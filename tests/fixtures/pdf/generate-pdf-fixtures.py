#!/usr/bin/env python3
"""Generate deterministic PDF fixtures for the M13 PDF ingestion slice.

Stdlib only (zlib, hashlib). No network, no system PDF tools: every
fixture is assembled byte by byte so the suite owns its inputs and the
expected extraction is known exactly. Run from the repo root:

    python3 tests/fixtures/pdf/generate-pdf-fixtures.py

Writes the fixtures plus MANIFEST.json (SHA-256 + expected per-page
text + expected slice outcome) into tests/fixtures/pdf/.

Reference check (not part of the app): `pdftotext` (poppler) is used
by the author to cross-check the normal fixtures' expected text.
"""

import base64
import hashlib
import json
import os
import struct
import zlib

OUT = os.path.join(os.path.dirname(os.path.abspath(__file__)))

PADDING = bytes([
    0x28, 0xBF, 0x4E, 0x5E, 0x4E, 0x75, 0x8A, 0x41, 0x64, 0x00,
    0x4E, 0x56, 0xFF, 0xFA, 0x01, 0x08, 0x2E, 0x2E, 0x00, 0xB6,
    0xD0, 0x68, 0x3E, 0x80, 0x2F, 0x0C, 0xA9, 0xFE, 0x64, 0x53,
    0x69, 0x7A,
])


def rc4(key: bytes, data: bytes) -> bytes:
    s = list(range(256))
    j = 0
    for i in range(256):
        j = (j + s[i] + key[i % len(key)]) % 256
        s[i], s[j] = s[j], s[i]
    out = bytearray()
    i = j = 0
    for byte in data:
        i = (i + 1) % 256
        j = (j + s[i]) % 256
        s[i], s[j] = s[j], s[i]
        out.append(byte ^ s[(s[i] + s[j]) % 256])
    return bytes(out)


def pad_password(password: bytes) -> bytes:
    return (password + PADDING)[:32]


class Pdf:
    """Minimal PDF 1.4 writer: catalog, page tree, Type1 fonts, streams."""

    def __init__(self):
        self.objects = {}  # id -> bytes (without the "N 0 obj" wrapper)

    def add(self, body: bytes) -> int:
        obj_id = max(self.objects, default=0) + 1
        self.objects[obj_id] = body
        return obj_id

    def reserve_page_tree(self):
        """Reserve object 2 for the /Pages tree before any add(), so
        auto-assigned ids never collide with the hardcoded tree id."""
        self.objects[2] = b"<< /Type /Pages >>"

    def stream(self, dictionary: bytes, data: bytes, compress: bool = True) -> bytes:
        if compress:
            data = zlib.compress(data)
            dictionary += b" /Filter /FlateDecode"
        return dictionary + b" /Length " + str(len(data)).encode() + b" >>\nstream\n" + data + b"\nendstream"

    def build(self, root_id: int, extra_trailer: bytes = b"", encrypt_id: int = 0) -> bytes:
        out = bytearray(b"%PDF-1.4\n%\xe2\xe3\xcf\xd3\n")
        offsets = {}
        for obj_id in sorted(self.objects):
            offsets[obj_id] = len(out)
            out += str(obj_id).encode() + b" 0 obj\n" + self.objects[obj_id] + b"\nendobj\n"
        xref_pos = len(out)
        count = max(self.objects) + 1
        out += b"xref\n0 " + str(count).encode() + b"\n"
        for obj_id in range(count):
            if obj_id in offsets:
                out += ("%010d 00000 n \n" % offsets[obj_id]).encode()
            else:
                # Object 0, and any id left free (object 1 is unused:
                # the page tree is hardcoded to object 2).
                out += b"0000000000 65535 f \n"
        out += (
            b"trailer\n<< /Size " + str(count).encode()
            + b" /Root " + str(root_id).encode() + b" 0 R"
            + extra_trailer + b" >>\nstartxref\n"
            + str(xref_pos).encode() + b"\n%%EOF\n"
        )
        return bytes(out)


def font_object(base_font=b"Helvetica", encoding=b"/WinAnsiEncoding", to_unicode=None):
    body = b"<< /Type /Font /Subtype /Type1 /BaseFont /" + base_font + b" /Encoding " + encoding
    if to_unicode is not None:
        body += b" /ToUnicode " + str(to_unicode).encode() + b" 0 R"
    return body + b" >>"


def page_objects(pdf: Pdf, pages):
    """pages: list of (content_bytes, extra_resources_dict_bytes).

    Returns the catalog object id. Each page gets a Helvetica /F1 font
    unless the page's resources override it.
    """
    pdf.reserve_page_tree()
    font_id = pdf.add(font_object())
    page_ids = []
    for content, extra_resources in pages:
        content_id = pdf.add(pdf.stream(b"<<", content))
        resources = b"<< /Font << /F1 " + str(font_id).encode() + b" 0 R >>"
        if extra_resources:
            resources += b" " + extra_resources
        resources += b" >>"
        page_id = pdf.add(
            b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] "
            b"/Resources " + resources
            + b" /Contents " + str(content_id).encode() + b" 0 R >>"
        )
        page_ids.append(page_id)
    kids = b" ".join(str(p).encode() + b" 0 R" for p in page_ids)
    pdf.objects[2] = (
        b"<< /Type /Pages /Kids [" + kids + b"] /Count " + str(len(page_ids)).encode() + b" >>"
    )
    catalog_id = pdf.add(b"<< /Type /Catalog /Pages 2 0 R >>")
    return catalog_id


def text_lines(lines, y_start=720, size=12, leading=16):
    """One BT block; each line is a Latin-1 string (WinAnsi byte = code point)."""
    parts = [b"BT /F1 " + str(size).encode() + b" Tf 72 " + str(y_start).encode() + b" Td"]
    for i, line in enumerate(lines):
        if i:
            parts.append(b"0 -" + str(leading).encode() + b" Td")
        parts.append(b"(" + line.encode("latin-1").replace(b"\\", b"\\\\").replace(b"(", b"\\(").replace(b")", b"\\)") + b") Tj")
    parts.append(b"ET")
    return b"\n".join(parts)


def lzw_encode(data: bytes) -> bytes:
    """PDF LZWDecode encoder, TIFF early-change (EarlyChange=1, the
    PDF default), MSB-first bit packing, min code size 8.

    Dictionary starts with the 256 single-byte entries; clear=256,
    EOD=257, first free code 258. Code width starts at 9 bits and
    grows to 12; with early change the width increases when the next
    code to assign reaches 2**width - 1. When the table fills (4096
    codes) a clear code is emitted and the dictionary resets. The
    module's weezl decoder (with_tiff_size_switch) is the reference
    for this encoding; the LZW fixtures' tests round-trip through it.
    """
    CLEAR, EOD = 256, 257
    out_bits = []  # (code, width) pairs, packed MSB-first below

    def emit(code, width):
        out_bits.append((code, width))

    dictionary = {bytes([b]): b for b in range(256)}
    next_code = 258
    width = 9
    emit(CLEAR, width)
    current = b""
    for byte in data:
        candidate = current + bytes([byte])
        if candidate in dictionary:
            current = candidate
            continue
        emit(dictionary[current], width)
        if next_code < 4096:
            dictionary[candidate] = next_code
            next_code += 1
            # Width bump: increase the code size when the entry just
            # added reaches (1 << width) - 1. The decoder (weezl,
            # TIFF early-change) bumps when it DERIVES an entry with
            # code >= (1 << width) - 2, and it derives each entry
            # one code AFTER the encoder adds it, so the encoder's
            # threshold is one higher — otherwise the widths desync
            # at the first bump (verified: bumping at -2 fails
            # weezl with InvalidCode at ~32k 'A's; bumping at -1
            # decodes cleanly).
            if next_code == (1 << width) and width < 12:
                width += 1
        else:
            emit(CLEAR, width)
            dictionary = {bytes([b]): b for b in range(256)}
            next_code = 258
            width = 9
        current = bytes([byte])
    if current:
        emit(dictionary[current], width)
    emit(EOD, width)
    # Pack MSB-first.
    acc, nbits, out = 0, 0, bytearray()
    for code, w in out_bits:
        acc = (acc << w) | code
        nbits += w
        while nbits >= 8:
            nbits -= 8
            out.append((acc >> nbits) & 0xFF)
    if nbits:
        out.append((acc << (8 - nbits)) & 0xFF)
    return bytes(out)


def a85_encode(data: bytes) -> bytes:
    """ASCII85 (Adobe variant without the <~ prefix): the bytes the
    /ASCII85Decode filter expects, ending with ~>."""
    return base64.a85encode(data, adobe=True)[2:]


def flate_two_block(prefix: bytes, suffix: bytes) -> bytes:
    """A FlateDecode (zlib) stream whose first deflate block is
    stored (literal) and whose remaining blocks are raw-deflate
    compressed. The stored block's bytes appear literally in the
    compressed stream, so a marker in the prefix (e.g. the bytes
    `endstream`) is visible to a raw scan for `endstream` — while
    the loader, which trusts /Length, takes the whole stream. The
    decoded output is prefix + suffix.

    Stored block layout: 1 byte header (BFINAL=0, BTYPE=00, padding
    0), then LEN (2 bytes LE), NLEN (2 bytes LE, one's complement),
    then the literal data. The suffix is a raw deflate stream
    (wbits=-15, no zlib wrapper) whose final block ends the stream.
    The whole deflate stream is wrapped in the zlib envelope
    (0x78 0x9C header + Adler32 of the uncompressed data), because
    FlateDecode is zlib, not raw deflate.
    """
    n = len(prefix)
    stored = b"\x00" + struct.pack("<H", n) + struct.pack("<H", (~n) & 0xFFFF) + prefix
    comp = zlib.compressobj(9, zlib.DEFLATED, -15)
    raw_deflate = stored + comp.compress(suffix) + comp.flush()
    adler = zlib.adler32(prefix + suffix) & 0xFFFFFFFF
    return b"\x78\x9c" + raw_deflate + struct.pack(">I", adler)


def flate_split_at_marker(raw: bytes, marker: bytes = b"endstream") -> bytes:
    """Compress `raw` as a two-block Flate stream, splitting just
    after the first occurrence of `marker` so the marker lands in
    the stored (literal) block. The decoded output is exactly `raw`."""
    idx = raw.find(marker)
    assert idx != -1, "marker not in raw content"
    split = idx + len(marker)
    return flate_two_block(raw[:split], raw[split:])


class Pdf15:
    """Minimal PDF 1.5 writer: normal objects, compressed objects
    (in one /ObjStm), and a cross-reference stream. Object 0 is
    free; the /Pages tree is object 2 by convention."""

    def __init__(self):
        self.normal = {}      # id -> bytes (full object body)
        self.compressed = {}  # id -> bytes (object body, no wrapper)

    def add_normal(self, body: bytes) -> int:
        obj_id = max(list(self.normal) + list(self.compressed), default=0) + 1
        self.normal[obj_id] = body
        return obj_id

    def add_compressed(self, body: bytes) -> int:
        obj_id = max(list(self.normal) + list(self.compressed), default=0) + 1
        self.compressed[obj_id] = body
        return obj_id

    def build(self, root_id: int, xref_stream_size: int = 0, objstm_compress=None) -> bytes:
        # Object stream (id = next free) holding the compressed objects.
        objstm_id = None
        if self.compressed:
            objstm_id = max(list(self.normal) + list(self.compressed), default=0) + 1
            index_parts, bodies, offset = [], bytearray(), 0
            for obj_id in sorted(self.compressed):
                body = self.compressed[obj_id]
                index_parts.append(str(obj_id).encode() + b" " + str(offset).encode() + b" ")
                bodies += body
                offset += len(body)
            index = b"".join(index_parts)
            raw = index + bytes(bodies)
            compress = objstm_compress or zlib.compress
            comp = compress(raw)
            self.normal[objstm_id] = (
                b"<< /Type /ObjStm /N " + str(len(self.compressed)).encode()
                + b" /First " + str(len(index)).encode()
                + b" /Filter /FlateDecode /Length " + str(len(comp)).encode()
                + b" >>\nstream\n" + comp + b"\nendstream"
            )
        # Layout: header, then normal objects in id order.
        out = bytearray(b"%PDF-1.5\n%\xe2\xe3\xcf\xd3\n")
        offsets = {}
        for obj_id in sorted(self.normal):
            offsets[obj_id] = len(out)
            out += str(obj_id).encode() + b" 0 obj\n" + self.normal[obj_id] + b"\nendobj\n"
        # Cross-reference stream: object ids 0..size-1, W=[1 4 2].
        size = xref_stream_size or (max(list(self.normal) + list(self.compressed), default=0) + 1)
        xref_id = max(list(self.normal), default=0) + 1
        entries = bytearray()
        for obj_id in range(size):
            if obj_id == 0:
                entries += bytes([0]) + (0).to_bytes(4, "big") + (65535).to_bytes(2, "big")
            elif obj_id in self.compressed:
                idx = sorted(self.compressed).index(obj_id)
                entries += bytes([2]) + objstm_id.to_bytes(4, "big") + idx.to_bytes(2, "big")
            elif obj_id in offsets:
                entries += bytes([1]) + offsets[obj_id].to_bytes(4, "big") + (0).to_bytes(2, "big")
            else:
                entries += bytes([0]) + (0).to_bytes(4, "big") + (0).to_bytes(2, "big")
        comp = zlib.compress(bytes(entries))
        xref_pos = len(out)
        out += (
            str(xref_id).encode() + b" 0 obj\n"
            b"<< /Type /XRef /Size " + str(size).encode()
            + b" /W [1 4 2] /Index [0 " + str(size).encode() + b"]"
            + b" /Root " + str(root_id).encode() + b" 0 R"
            + b" /Filter /FlateDecode /Length " + str(len(comp)).encode()
            + b" >>\nstream\n" + comp + b"\nendstream\nendobj\n"
        )
        out += b"startxref\n" + str(xref_pos).encode() + b"\n%%EOF\n"
        return bytes(out)


def write(name: str, data: bytes):
    with open(os.path.join(OUT, name), "wb") as f:
        f.write(data)
    return name

# ---------------------------------------------------------------- fixtures

def normal_3page():
    pdf = Pdf()
    pages = [
        (text_lines([
            "Harbor Ledger",
            "The bramblewick shipment cleared customs on Tuesday.",
            "Three crates of copper fittings, two of rope.",
        ]), b""),
        (text_lines([
            "Second page",
            "The aurora storage annex holds the spare manifests.",
            "Caf\xe9 service ends at noon.",  # e-acute in WinAnsi (0xE9)
        ]), b""),
        (text_lines([
            "Third page",
            "Totals: 41 crates, 12 pallets, 1 lighthouse keeper.",
        ]), b""),
    ]
    root = page_objects(pdf, pages)
    return pdf.build(root), [
        "Harbor Ledger\nThe bramblewick shipment cleared customs on Tuesday.\nThree crates of copper fittings, two of rope.",
        "Second page\nThe aurora storage annex holds the spare manifests.\nCafé service ends at noon.",
        "Third page\nTotals: 41 crates, 12 pallets, 1 lighthouse keeper.",
    ]


def unicode_cmap():
    """One page whose font maps raw codes 0x01/0x02/0x03 through a
    ToUnicode CMap to Greek alpha/beta and the euro sign. A naive
    byte reader returns control bytes; a correct extractor returns
    the mapped Unicode text."""
    pdf = Pdf()
    pdf.reserve_page_tree()
    cmap = (
        b"/CIDInit /ProcSet findresource begin\n12 dict begin\nbegincmap\n"
        b"/CIDSystemInfo << /Registry (Adobe) /Ordering (UCS) /Supplement 0 >> def\n"
        b"/CMapName /Adobe-Identity-UCS def\n/CMapType 2 def\n"
        b"1 begincodespacerange\n<00> <FF>\nendcodespacerange\n"
        b"3 beginbfchar\n<01> <03B1>\n<02> <03B2>\n<03> <20AC>\nendbfchar\n"
        b"endcmap\nCMapName currentdict /CMap defineresource pop\nend\nend"
    )
    cmap_id = pdf.add(pdf.stream(b"<<", cmap))
    font_id = pdf.add(
        b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica "
        b"/Encoding << /Type /Encoding /BaseEncoding /WinAnsiEncoding "
        b"/Differences [1 /Alpha /Beta /Euro] >> "
        b"/ToUnicode " + str(cmap_id).encode() + b" 0 R >>"
    )
    content = b"BT /F1 12 Tf 72 720 Td (\\001\\002\\003) Tj ET"
    content_id = pdf.add(pdf.stream(b"<<", content))
    page_id = pdf.add(
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] "
        b"/Resources << /Font << /F1 " + str(font_id).encode() + b" 0 R >> >> "
        b"/Contents " + str(content_id).encode() + b" 0 R >>"
    )
    pdf.objects[2] = b"<< /Type /Pages /Kids [" + str(page_id).encode() + b" 0 R] /Count 1 >>"
    root = pdf.add(b"<< /Type /Catalog /Pages 2 0 R >>")
    return pdf.build(root), ["\u03b1\u03b2\u20ac"]


def scanned_image():
    """Pages with only an image XObject and no text operators: a
    scanned document. The extractor must find zero text."""
    pdf = Pdf()
    pdf.reserve_page_tree()
    image = pdf.add(pdf.stream(
        b"<< /Type /XObject /Subtype /Image /Width 2 /Height 2 "
        b"/ColorSpace /DeviceRGB /BitsPerComponent 8",
        bytes([255, 0, 0, 0, 255, 0, 0, 0, 255, 255, 255, 255]),
    ))
    content = b"q 200 0 0 200 72 500 cm /Im1 Do Q"
    content_id = pdf.add(pdf.stream(b"<<", content))
    font_id = pdf.add(font_object())
    resources = (
        b"<< /Font << /F1 " + str(font_id).encode() + b" 0 R >> "
        b"/XObject << /Im1 " + str(image).encode() + b" 0 R >> >>"
    )
    page_id = pdf.add(
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] "
        b"/Resources " + resources
        + b" /Contents " + str(content_id).encode() + b" 0 R >>"
    )
    pdf.objects[2] = b"<< /Type /Pages /Kids [" + str(page_id).encode() + b" 0 R] /Count 1 >>"
    root = pdf.add(b"<< /Type /Catalog /Pages 2 0 R >>")
    return pdf.build(root), []


def empty_text():
    pdf = Pdf()
    root = page_objects(pdf, [(b"", b"")])
    return pdf.build(root), [""]


def too_many_pages():
    pdf = Pdf()
    pages = [(text_lines(["Page %d." % (i + 1)]), b"") for i in range(101)]
    root = page_objects(pdf, pages)
    return pdf.build(root), ["Page %d." % (i + 1) for i in range(101)]


def oversized_text():
    """5 pages x 7,000 chars: each page under the 8,000-char page cap,
    total 35,000 over the 32,000-char document cap. Raw size stays
    well under the raw cap, so the refusal must come from extraction."""
    pdf = Pdf()
    pages = [(text_lines(["b" * 100] * 70, y_start=780, size=8, leading=10), b"") for _ in range(5)]
    root = page_objects(pdf, pages)
    return pdf.build(root), ["\n".join(["b" * 100] * 70)] * 5


def oversized_raw_pdf():
    pdf = Pdf()
    pages = [(text_lines(["Harbor Ledger", "Small page."]), b"")]
    root = page_objects(pdf, pages)
    big_id = pdf.add(
        b"<< /Length 1100000 >>\nstream\n" + b"x" * 1100000 + b"\nendstream"
    )
    _ = big_id
    return pdf.build(root), ["Harbor Ledger\nSmall page."]


def decompression_bomb():
    """One page whose FlateDecode content stream expands to ~20 MB of
    'A' text: raw file ~25 KB, extracted page far over every cap.
    The spike must show the extractor bounds this (or the slice adds
    a bounded decode path before integration)."""
    pdf = Pdf()
    huge = b"BT /F1 12 Tf 72 720 Td (" + b"A" * 20_000_000 + b") Tj ET"
    root = page_objects(pdf, [(huge, b"")])
    return pdf.build(root), ["A" * 20_000_000]


def encrypted_pdf():
    """Standard security handler, R=2, 40-bit RC4, user password
    'secret', empty owner password. Only the content stream carries
    encrypted bytes; names and numbers are never encrypted."""
    user_pwd = b"secret"
    owner_pwd = b""
    file_id = bytes.fromhex("0123456789abcdeffedcba9876543210")
    owner_key = hashlib.md5(pad_password(owner_pwd)).digest()
    o_value = rc4(owner_key, pad_password(user_pwd))
    perms = struct.pack("<i", -4)
    file_key = hashlib.md5(pad_password(user_pwd) + o_value + perms + file_id).digest()[:5]
    u_value = rc4(file_key, PADDING)

    def object_key(obj_id: int) -> bytes:
        return hashlib.md5(file_key + struct.pack("<I", obj_id)[:3] + struct.pack("<H", 0)).digest()[:10]

    pdf = Pdf()
    pdf.reserve_page_tree()
    font_id = pdf.add(font_object())
    content = text_lines(["Secret harbor ledger.", "Do not file under bramblewick."])
    content_id = max(pdf.objects, default=0) + 1
    compressed = zlib.compress(content)
    encrypted_stream = rc4(object_key(content_id), compressed)
    pdf.objects[content_id] = (
        b"<< /Filter /FlateDecode /Length " + str(len(encrypted_stream)).encode()
        + b" >>\nstream\n" + encrypted_stream + b"\nendstream"
    )
    page_id = pdf.add(
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] "
        b"/Resources << /Font << /F1 " + str(font_id).encode() + b" 0 R >> >> "
        b"/Contents " + str(content_id).encode() + b" 0 R >>"
    )
    pdf.objects[2] = b"<< /Type /Pages /Kids [" + str(page_id).encode() + b" 0 R] /Count 1 >>"
    root = pdf.add(b"<< /Type /Catalog /Pages 2 0 R >>")
    encrypt_id = pdf.add(
        b"<< /Filter /Standard /V 1 /R 2 /Length 40 /P -4 /O <"
        + o_value.hex().encode() + b"> /U <" + u_value.hex().encode() + b"> >>"
    )
    trailer = (
        b" /Encrypt " + str(encrypt_id).encode() + b" 0 R"
        + b" /ID [<" + file_id.hex().encode() + b"> <" + file_id.hex().encode() + b">]"
    )
    return pdf.build(root, extra_trailer=trailer), ["Secret harbor ledger.\nDo not file under bramblewick."]


def object_stream_ok():
    """PDF 1.5: the font and the /Pages tree live inside a compressed
    object stream; the content stream is a normal object; the xref is
    a stream. A bounded extractor must import this (the object
    stream decodes to a few hundred bytes)."""
    pdf = Pdf15()
    # Reserve id 2 for the /Pages tree (a compressed object) BEFORE
    # adding any normal object, so no normal object collides with it.
    pdf.compressed[2] = b""
    font_id = pdf.add_normal(font_object())
    content = text_lines(["Harbor Ledger", "The object-stream shipment cleared customs."])
    content_id = pdf.add_normal(pdf_stream_raw(b"<<", content))
    page_id = pdf.add_normal(
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] "
        b"/Resources << /Font << /F1 " + str(font_id).encode() + b" 0 R >> >> "
        b"/Contents " + str(content_id).encode() + b" 0 R >>"
    )
    # The /Pages tree is a compressed object (id 2, reserved above).
    pdf.compressed[2] = (
        b"<< /Type /Pages /Kids [" + str(page_id).encode() + b" 0 R] /Count 1 >>"
    )
    root = pdf.add_normal(b"<< /Type /Catalog /Pages 2 0 R >>")
    return pdf.build(root), ["Harbor Ledger\nThe object-stream shipment cleared customs."]


def object_stream_bomb():
    """Classic-xref PDF with one unreferenced /ObjStm whose FlateDecode
    data expands to ~20 MB (a single 20 MB string object inside it).
    lopdf decompresses every ObjStm at load, so the bomb detonates in
    Document::load_mem itself — the pre-load scan must refuse it first."""
    pdf = Pdf()
    pdf.reserve_page_tree()
    font_id = pdf.add(font_object())
    root = page_objects(pdf, [(text_lines(["Harbor Ledger", "Small page."]), b"")])
    index = b"99 0 "
    body = b"(" + b"A" * 20_000_000 + b")"
    raw = index + body
    comp = zlib.compress(raw)
    objstm = pdf.add(
        b"<< /Type /ObjStm /N 1 /First " + str(len(index)).encode()
        + b" /Filter /FlateDecode /Length " + str(len(comp)).encode()
        + b" >>\nstream\n" + comp + b"\nendstream"
    )
    _ = objstm
    return pdf.build(root), []


def xref_stream_ok():
    """PDF 1.5 with a cross-reference stream and all objects normal
    (no object stream). A bounded extractor must import this."""
    pdf = Pdf15()
    root = page_objects_15(pdf, [(text_lines(["Harbor Ledger", "The xref-stream shipment cleared customs."]), b"")])
    return pdf.build(root), ["Harbor Ledger\nThe xref-stream shipment cleared customs."]


def xref_stream_bomb():
    """PDF 1.5 whose xref stream decodes to ~21 MB (3,000,000 entries
    x 7 bytes, mostly free). The pre-load scan must refuse it before
    lopdf parses the reference table."""
    pdf = Pdf15()
    root = page_objects_15(pdf, [(text_lines(["Harbor Ledger", "Small page."]), b"")])
    return pdf.build(root, xref_stream_size=3_000_000), []


def chained_filter_ok():
    """Content stream with /Filter [/ASCII85Decode /FlateDecode]: the
    ASCII85 layer wraps the Flate layer. Must import."""
    pdf = Pdf()
    pdf.reserve_page_tree()
    font_id = pdf.add(font_object())
    content = text_lines(["Harbor Ledger", "The chained-filter shipment cleared customs."])
    data = a85_encode(zlib.compress(content))
    content_id = pdf.add(
        b"<< /Filter [/ASCII85Decode /FlateDecode] /Length " + str(len(data)).encode()
        + b" >>\nstream\n" + data + b"\nendstream"
    )
    page_id = pdf.add(
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] "
        b"/Resources << /Font << /F1 " + str(font_id).encode() + b" 0 R >> >> "
        b"/Contents " + str(content_id).encode() + b" 0 R >>"
    )
    pdf.objects[2] = b"<< /Type /Pages /Kids [" + str(page_id).encode() + b" 0 R] /Count 1 >>"
    root = pdf.add(b"<< /Type /Catalog /Pages 2 0 R >>")
    return pdf.build(root), ["Harbor Ledger\nThe chained-filter shipment cleared customs."]


def chained_filter_bomb():
    """Content stream with /Filter [/FlateDecode /FlateDecode] whose
    final decoded size is ~20 MB. The cumulative budget must count
    both stages: the inner Flate expansion is what trips it."""
    pdf = Pdf()
    pdf.reserve_page_tree()
    font_id = pdf.add(font_object())
    huge = b"BT /F1 12 Tf 72 720 Td (" + b"A" * 20_000_000 + b") Tj ET"
    data = zlib.compress(zlib.compress(huge))
    content_id = pdf.add(
        b"<< /Filter [/FlateDecode /FlateDecode] /Length " + str(len(data)).encode()
        + b" >>\nstream\n" + data + b"\nendstream"
    )
    page_id = pdf.add(
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] "
        b"/Resources << /Font << /F1 " + str(font_id).encode() + b" 0 R >> >> "
        b"/Contents " + str(content_id).encode() + b" 0 R >>"
    )
    pdf.objects[2] = b"<< /Type /Pages /Kids [" + str(page_id).encode() + b" 0 R] /Count 1 >>"
    root = pdf.add(b"<< /Type /Catalog /Pages 2 0 R >>")
    return pdf.build(root), []


def lzw_ok():
    """Content stream with /Filter /LZWDecode (TIFF early-change).
    Must import; this also validates the fixture generator's LZW
    encoder against the module's weezl decoder."""
    pdf = Pdf()
    pdf.reserve_page_tree()
    font_id = pdf.add(font_object())
    content = text_lines(["Harbor Ledger", "The lzw shipment cleared customs."])
    data = lzw_encode(content)
    content_id = pdf.add(
        b"<< /Filter /LZWDecode /Length " + str(len(data)).encode()
        + b" >>\nstream\n" + data + b"\nendstream"
    )
    page_id = pdf.add(
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] "
        b"/Resources << /Font << /F1 " + str(font_id).encode() + b" 0 R >> >> "
        b"/Contents " + str(content_id).encode() + b" 0 R >>"
    )
    pdf.objects[2] = b"<< /Type /Pages /Kids [" + str(page_id).encode() + b" 0 R] /Count 1 >>"
    root = pdf.add(b"<< /Type /Catalog /Pages 2 0 R >>")
    return pdf.build(root), ["Harbor Ledger\nThe lzw shipment cleared customs."]


def lzw_bomb():
    """Content stream with /Filter /LZWDecode expanding to ~20 MB.
    The capped weezl path must refuse it (Bomb), not decode it."""
    pdf = Pdf()
    pdf.reserve_page_tree()
    font_id = pdf.add(font_object())
    huge = b"BT /F1 12 Tf 72 720 Td (" + b"A" * 20_000_000 + b") Tj ET"
    data = lzw_encode(huge)
    content_id = pdf.add(
        b"<< /Filter /LZWDecode /Length " + str(len(data)).encode()
        + b" >>\nstream\n" + data + b"\nendstream"
    )
    page_id = pdf.add(
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] "
        b"/Resources << /Font << /F1 " + str(font_id).encode() + b" 0 R >> >> "
        b"/Contents " + str(content_id).encode() + b" 0 R >>"
    )
    pdf.objects[2] = b"<< /Type /Pages /Kids [" + str(page_id).encode() + b" 0 R] /Count 1 >>"
    root = pdf.add(b"<< /Type /Catalog /Pages 2 0 R >>")
    return pdf.build(root), []


def nested_form_ok():
    """Page -> Form /Fm1 -> Form /Fm2 -> text. The pre-scan must walk
    forms recursively; pdf-extract must surface the innermost text."""
    pdf = Pdf()
    pdf.reserve_page_tree()
    font_id = pdf.add(font_object())
    inner_text = text_lines(["Harbor Ledger", "The nested-form shipment cleared customs."])
    inner_id = pdf.add(pdf.stream(
        b"<< /Type /XObject /Subtype /Form /BBox [0 0 612 792] "
        b"/Resources << /Font << /F1 " + str(font_id).encode() + b" 0 R >> >>",
        inner_text,
    ))
    outer_id = pdf.add(pdf.stream(
        b"<< /Type /XObject /Subtype /Form /BBox [0 0 612 792] "
        b"/Resources << /XObject << /Fm2 " + str(inner_id).encode() + b" 0 R >> >>",
        b"q /Fm2 Do Q",
    ))
    content_id = pdf.add(pdf.stream(b"<<", b"q /Fm1 Do Q"))
    page_id = pdf.add(
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] "
        b"/Resources << /Font << /F1 " + str(font_id).encode() + b" 0 R >> "
        b"/XObject << /Fm1 " + str(outer_id).encode() + b" 0 R >> >> "
        b"/Contents " + str(content_id).encode() + b" 0 R >>"
    )
    pdf.objects[2] = b"<< /Type /Pages /Kids [" + str(page_id).encode() + b" 0 R] /Count 1 >>"
    root = pdf.add(b"<< /Type /Catalog /Pages 2 0 R >>")
    return pdf.build(root), ["Harbor Ledger\nThe nested-form shipment cleared customs."]


def nested_form_bomb():
    """Page -> Form /Fm1 -> Form /Fm2, where /Fm2's content stream is
    a FlateDecode bomb (~20 MB). The recursive pre-scan must find it
    and refuse it; a one-level walk would miss it."""
    pdf = Pdf()
    pdf.reserve_page_tree()
    font_id = pdf.add(font_object())
    huge = b"BT /F1 12 Tf 72 720 Td (" + b"A" * 20_000_000 + b") Tj ET"
    inner_id = pdf.add(pdf.stream(
        b"<< /Type /XObject /Subtype /Form /BBox [0 0 612 792] "
        b"/Resources << /Font << /F1 " + str(font_id).encode() + b" 0 R >> >>",
        huge,
    ))
    outer_id = pdf.add(pdf.stream(
        b"<< /Type /XObject /Subtype /Form /BBox [0 0 612 792] "
        b"/Resources << /XObject << /Fm2 " + str(inner_id).encode() + b" 0 R >> >>",
        b"q /Fm2 Do Q",
    ))
    content_id = pdf.add(pdf.stream(b"<<", b"q /Fm1 Do Q"))
    page_id = pdf.add(
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] "
        b"/Resources << /Font << /F1 " + str(font_id).encode() + b" 0 R >> "
        b"/XObject << /Fm1 " + str(outer_id).encode() + b" 0 R >> >> "
        b"/Contents " + str(content_id).encode() + b" 0 R >>"
    )
    pdf.objects[2] = b"<< /Type /Pages /Kids [" + str(page_id).encode() + b" 0 R] /Count 1 >>"
    root = pdf.add(b"<< /Type /Catalog /Pages 2 0 R >>")
    return pdf.build(root), []


def indirect_resources_ok():
    """Page whose /Resources is an indirect object (not inline).
    The indirect dict holds a font and a small Form. The old
    pre-scan read only a direct /Resources on the page dict, so it
    missed the Form; the extractor resolves the indirect dict
    (get_inherited) and decodes the Form."""
    pdf = Pdf()
    pdf.reserve_page_tree()
    font_id = pdf.add(font_object())
    form_text = text_lines(["Harbor Ledger", "The indirect-resources shipment cleared customs."])
    form_id = pdf.add(pdf.stream(
        b"<< /Type /XObject /Subtype /Form /BBox [0 0 612 792] "
        b"/Resources << /Font << /F1 " + str(font_id).encode() + b" 0 R >> >>",
        form_text,
    ))
    resources_id = pdf.add(
        b"<< /Font << /F1 " + str(font_id).encode() + b" 0 R >> "
        b"/XObject << /Fm1 " + str(form_id).encode() + b" 0 R >> >>"
    )
    content_id = pdf.add(pdf.stream(b"<<", b"q /Fm1 Do Q"))
    page_id = pdf.add(
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] "
        b"/Resources " + str(resources_id).encode() + b" 0 R "
        b"/Contents " + str(content_id).encode() + b" 0 R >>"
    )
    pdf.objects[2] = b"<< /Type /Pages /Kids [" + str(page_id).encode() + b" 0 R] /Count 1 >>"
    root = pdf.add(b"<< /Type /Catalog /Pages 2 0 R >>")
    return pdf.build(root), ["Harbor Ledger\nThe indirect-resources shipment cleared customs."]


def indirect_resources_bomb():
    """Same shape as indirect-resources-ok, but the Form reached
    only through the indirect /Resources is a FlateDecode bomb
    (~20 MB). The old pre-scan never decoded it; the extractor
    decoded it uncapped during extraction."""
    pdf = Pdf()
    pdf.reserve_page_tree()
    font_id = pdf.add(font_object())
    huge = b"BT /F1 12 Tf 72 720 Td (" + b"A" * 20_000_000 + b") Tj ET"
    form_id = pdf.add(pdf.stream(
        b"<< /Type /XObject /Subtype /Form /BBox [0 0 612 792] "
        b"/Resources << /Font << /F1 " + str(font_id).encode() + b" 0 R >> >>",
        huge,
    ))
    resources_id = pdf.add(
        b"<< /Font << /F1 " + str(font_id).encode() + b" 0 R >> "
        b"/XObject << /Fm1 " + str(form_id).encode() + b" 0 R >> >>"
    )
    content_id = pdf.add(pdf.stream(b"<<", b"q /Fm1 Do Q"))
    page_id = pdf.add(
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] "
        b"/Resources " + str(resources_id).encode() + b" 0 R "
        b"/Contents " + str(content_id).encode() + b" 0 R >>"
    )
    pdf.objects[2] = b"<< /Type /Pages /Kids [" + str(page_id).encode() + b" 0 R] /Count 1 >>"
    root = pdf.add(b"<< /Type /Catalog /Pages 2 0 R >>")
    return pdf.build(root), []


def inherited_resources_ok():
    """Page with NO /Resources of its own; the /Pages node carries
    /Resources (font + small Form). The extractor inherits them
    (get_inherited walks up the page tree); the old pre-scan read
    only the page dict and missed the Form."""
    pdf = Pdf()
    pdf.reserve_page_tree()
    font_id = pdf.add(font_object())
    form_text = text_lines(["Harbor Ledger", "The inherited-resources shipment cleared customs."])
    form_id = pdf.add(pdf.stream(
        b"<< /Type /XObject /Subtype /Form /BBox [0 0 612 792] "
        b"/Resources << /Font << /F1 " + str(font_id).encode() + b" 0 R >> >>",
        form_text,
    ))
    content_id = pdf.add(pdf.stream(b"<<", b"q /Fm1 Do Q"))
    page_id = pdf.add(
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] "
        b"/Contents " + str(content_id).encode() + b" 0 R >>"
    )
    pdf.objects[2] = (
        b"<< /Type /Pages /Kids [" + str(page_id).encode() + b" 0 R] /Count 1 "
        b"/Resources << /Font << /F1 " + str(font_id).encode() + b" 0 R >> "
        b"/XObject << /Fm1 " + str(form_id).encode() + b" 0 R >> >> >>"
    )
    root = pdf.add(b"<< /Type /Catalog /Pages 2 0 R >>")
    return pdf.build(root), ["Harbor Ledger\nThe inherited-resources shipment cleared customs."]


def inherited_resources_bomb():
    """Same shape as inherited-resources-ok, but the Form reached
    only through the inherited /Resources is a FlateDecode bomb
    (~20 MB). The old pre-scan never decoded it."""
    pdf = Pdf()
    pdf.reserve_page_tree()
    font_id = pdf.add(font_object())
    huge = b"BT /F1 12 Tf 72 720 Td (" + b"A" * 20_000_000 + b") Tj ET"
    form_id = pdf.add(pdf.stream(
        b"<< /Type /XObject /Subtype /Form /BBox [0 0 612 792] "
        b"/Resources << /Font << /F1 " + str(font_id).encode() + b" 0 R >> >>",
        huge,
    ))
    content_id = pdf.add(pdf.stream(b"<<", b"q /Fm1 Do Q"))
    page_id = pdf.add(
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] "
        b"/Contents " + str(content_id).encode() + b" 0 R >>"
    )
    pdf.objects[2] = (
        b"<< /Type /Pages /Kids [" + str(page_id).encode() + b" 0 R] /Count 1 "
        b"/Resources << /Font << /F1 " + str(font_id).encode() + b" 0 R >> "
        b"/XObject << /Fm1 " + str(form_id).encode() + b" 0 R >> >> >>"
    )
    root = pdf.add(b"<< /Type /Catalog /Pages 2 0 R >>")
    return pdf.build(root), []


def tounicode_cmap_text(entries):
    """A ToUnicode CMap body with `entries` bfchar lines. Each line
    maps code 0x01 to U+03B1 (alpha); the repetition compresses to
    almost nothing, so a large entry count is a small raw stream
    that expands to many megabytes."""
    lines = [b"/CIDInit /ProcSet findresource begin\n12 dict begin\nbegincmap\n"
             b"/CIDSystemInfo << /Registry (Adobe) /Ordering (UCS) /Supplement 0 >> def\n"
             b"/CMapName /Adobe-Identity-UCS def\n/CMapType 2 def\n"
             b"1 begincodespacerange\n<00> <FF>\nendcodespacerange\n"
             + str(entries).encode() + b" beginbfchar\n"]
    lines.append(b"<01> <03B1>\n" * entries)
    lines.append(b"endbfchar\nendcmap\nCMapName currentdict /CMap defineresource pop\nend\nend")
    return b"".join(lines)


def tounicode_compressed_ok():
    """A font whose /ToUnicode is a FlateDecode-compressed CMap with
    256 bfchar entries (~3 KB decoded). The old pre-scan never
    measured CMap streams; the extractor decodes them (get_contents)
    to map text. Must import, charging the small CMap."""
    pdf = Pdf()
    pdf.reserve_page_tree()
    cmap_id = pdf.add(pdf.stream(b"<<", tounicode_cmap_text(256)))
    font_id = pdf.add(
        b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica "
        b"/Encoding << /Type /Encoding /BaseEncoding /WinAnsiEncoding "
        b"/Differences [1 /Alpha /Beta /Euro] >> "
        b"/ToUnicode " + str(cmap_id).encode() + b" 0 R >>"
    )
    content = b"BT /F1 12 Tf 72 720 Td (\\001) Tj ET"
    content_id = pdf.add(pdf.stream(b"<<", content))
    page_id = pdf.add(
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] "
        b"/Resources << /Font << /F1 " + str(font_id).encode() + b" 0 R >> >> "
        b"/Contents " + str(content_id).encode() + b" 0 R >>"
    )
    pdf.objects[2] = b"<< /Type /Pages /Kids [" + str(page_id).encode() + b" 0 R] /Count 1 >>"
    root = pdf.add(b"<< /Type /Catalog /Pages 2 0 R >>")
    return pdf.build(root), ["\u03b1"]


def tounicode_compressed_bomb():
    """A font whose /ToUnicode is a FlateDecode-compressed CMap with
    1,700,000 bfchar entries (~20 MB decoded, ~a few KB raw). The
    old pre-scan never measured CMap streams, so the extractor
    decoded ~20 MB uncapped while building the font (and then parsed
    1.7M entries). The repaired pre-scan charges it and refuses it."""
    pdf = Pdf()
    pdf.reserve_page_tree()
    cmap_id = pdf.add(pdf.stream(b"<<", tounicode_cmap_text(1_700_000)))
    font_id = pdf.add(
        b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica "
        b"/Encoding << /Type /Encoding /BaseEncoding /WinAnsiEncoding "
        b"/Differences [1 /Alpha /Beta /Euro] >> "
        b"/ToUnicode " + str(cmap_id).encode() + b" 0 R >>"
    )
    content = b"BT /F1 12 Tf 72 720 Td (\\001) Tj ET"
    content_id = pdf.add(pdf.stream(b"<<", content))
    page_id = pdf.add(
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] "
        b"/Resources << /Font << /F1 " + str(font_id).encode() + b" 0 R >> >> "
        b"/Contents " + str(content_id).encode() + b" 0 R >>"
    )
    pdf.objects[2] = b"<< /Type /Pages /Kids [" + str(page_id).encode() + b" 0 R] /Count 1 >>"
    root = pdf.add(b"<< /Type /Catalog /Pages 2 0 R >>")
    return pdf.build(root), []


def repeated_form_ok():
    """The same small Form invoked once on each of two pages. The
    old pre-scan's visited set charged it once across both pages;
    the extractor decodes it once per page."""
    pdf = Pdf()
    pdf.reserve_page_tree()
    font_id = pdf.add(font_object())
    form_text = text_lines(["Harbor Ledger", "The repeated-form shipment cleared customs."])
    form_id = pdf.add(pdf.stream(
        b"<< /Type /XObject /Subtype /Form /BBox [0 0 612 792] "
        b"/Resources << /Font << /F1 " + str(font_id).encode() + b" 0 R >> >>",
        form_text,
    ))
    page_ids = []
    for _ in range(2):
        content_id = pdf.add(pdf.stream(b"<<", b"q /Fm1 Do Q"))
        page_id = pdf.add(
            b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] "
            b"/Resources << /Font << /F1 " + str(font_id).encode() + b" 0 R >> "
            b"/XObject << /Fm1 " + str(form_id).encode() + b" 0 R >> >> "
            b"/Contents " + str(content_id).encode() + b" 0 R >>"
        )
        page_ids.append(page_id)
    kids = b" ".join(str(p).encode() + b" 0 R" for p in page_ids)
    pdf.objects[2] = b"<< /Type /Pages /Kids [" + kids + b"] /Count 2 >>"
    root = pdf.add(b"<< /Type /Catalog /Pages 2 0 R >>")
    return pdf.build(root), ["Harbor Ledger\nThe repeated-form shipment cleared customs."] * 2


def repeated_form_bomb():
    """One Form (~1 MB decoded text) invoked 20 times on one page.
    One decode fits the 16 MiB budget, but 20 invocations cost
    ~20 MB. The old pre-scan charged it once (visited set); the
    extractor decodes it once per `Do` (no caching)."""
    pdf = Pdf()
    pdf.reserve_page_tree()
    font_id = pdf.add(font_object())
    form_text = b"BT /F1 12 Tf 72 720 Td (" + b"A" * 1_000_000 + b") Tj ET"
    form_id = pdf.add(pdf.stream(
        b"<< /Type /XObject /Subtype /Form /BBox [0 0 612 792] "
        b"/Resources << /Font << /F1 " + str(font_id).encode() + b" 0 R >> >>",
        form_text,
    ))
    content = b"\n".join([b"q /Fm1 Do Q"] * 20)
    content_id = pdf.add(pdf.stream(b"<<", content))
    page_id = pdf.add(
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] "
        b"/Resources << /Font << /F1 " + str(font_id).encode() + b" 0 R >> "
        b"/XObject << /Fm1 " + str(form_id).encode() + b" 0 R >> >> "
        b"/Contents " + str(content_id).encode() + b" 0 R >>"
    )
    pdf.objects[2] = b"<< /Type /Pages /Kids [" + str(page_id).encode() + b" 0 R] /Count 1 >>"
    root = pdf.add(b"<< /Type /Catalog /Pages 2 0 R >>")
    return pdf.build(root), []


def stream_boundary_ok():
    """PDF 1.5 whose /ObjStm is compressed with a two-block Flate
    stream: a stored block carrying the literal bytes `endstream`
    (inside a string object), then a zlib block. The loader trusts
    /Length and decodes the whole stream; a scan for `endstream`
    truncates at the literal marker and either under-measures or
    fails the truncated decode. The repaired scan uses /Length.
    The /Pages tree is a compressed object; a second compressed
    object is the string `(endstream marker)`."""
    pdf = Pdf15()
    # Reserve object 2 for the /Pages tree before any add_normal, so
    # auto-assigned ids never collide with the hardcoded tree id
    # (the same collision the Pdf builder's reserve_page_tree fixes).
    pdf.compressed[2] = b""
    # Page + content first (normal objects), so ids are stable.
    font_id = pdf.add_normal(font_object())
    content_id = pdf.add_normal(pdf_stream_raw(
        b"<<", text_lines(["Harbor Ledger", "The stream-boundary shipment cleared customs."])))
    page_id = pdf.add_normal(
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] "
        b"/Resources << /Font << /F1 " + str(font_id).encode() + b" 0 R >> >> "
        b"/Contents " + str(content_id).encode() + b" 0 R >>"
    )
    # Compressed objects: the /Pages tree (id 2, by convention) and
    # a string object whose body contains the literal `endstream`.
    pdf.compressed[2] = (
        b"<< /Type /Pages /Kids [" + str(page_id).encode() + b" 0 R] /Count 1 >>"
    )
    pdf.compressed[99] = b"(endstream marker)"
    root = pdf.add_normal(b"<< /Type /Catalog /Pages 2 0 R >>")
    data = pdf.build(root, objstm_compress=flate_split_at_marker)
    return data, ["Harbor Ledger\nThe stream-boundary shipment cleared customs."]


def pdf_stream_raw(dictionary: bytes, data: bytes) -> bytes:
    """A FlateDecode stream object body (used by the Pdf15 builder,
    whose stream() helper lives on Pdf)."""
    comp = zlib.compress(data)
    return (
        dictionary + b" /Filter /FlateDecode /Length " + str(len(comp)).encode()
        + b" >>\nstream\n" + comp + b"\nendstream"
    )


def page_objects_15(pdf: Pdf15, pages):
    """Like page_objects but for the Pdf15 builder (object 2 is the
    /Pages tree; ids are assigned by add_normal)."""
    pdf.normal[2] = b"<< /Type /Pages >>"
    font_id = pdf.add_normal(font_object())
    page_ids = []
    for content, extra_resources in pages:
        content_id = pdf.add_normal(pdf_stream_raw(b"<<", content))
        resources = b"<< /Font << /F1 " + str(font_id).encode() + b" 0 R >>"
        if extra_resources:
            resources += b" " + extra_resources
        resources += b" >>"
        page_id = pdf.add_normal(
            b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] "
            b"/Resources " + resources
            + b" /Contents " + str(content_id).encode() + b" 0 R >>"
        )
        page_ids.append(page_id)
    kids = b" ".join(str(p).encode() + b" 0 R" for p in page_ids)
    pdf.normal[2] = (
        b"<< /Type /Pages /Kids [" + kids + b"] /Count " + str(len(page_ids)).encode() + b" >>"
    )
    return pdf.add_normal(b"<< /Type /Catalog /Pages 2 0 R >>")


def main():
    manifest = {"fixtures": {}}

    def add(name, data, pages, outcome, notes=""):
        write(name, data)
        manifest["fixtures"][name] = {
            "sha256": hashlib.sha256(data).hexdigest(),
            "bytes": len(data),
            "pages": len(pages),
            "expected_pages": pages,
            "expected_outcome": outcome,
            "notes": notes,
        }
        print(f"{name}: {len(data)} bytes, {len(pages)} pages -> {outcome}")

    normal, normal_pages = normal_3page()
    add("normal-3page.pdf", normal, normal_pages, "imported",
        "3 pages, WinAnsi e-acute on page 2.")
    uni, uni_pages = unicode_cmap()
    add("unicode-cmap.pdf", uni, uni_pages, "imported",
        "ToUnicode CMap: raw codes 01/02/03 must extract as alpha/beta/euro.")
    scan, _ = scanned_image()
    add("scanned-image.pdf", scan, [], "refused: no text layer",
        "Image-only page; OCR is a separate slice.")
    empty, _ = empty_text()
    add("empty-text.pdf", empty, [""], "refused: empty",
        "Valid PDF, blank page, zero text.")
    many, many_pages = too_many_pages()
    add("too-many-pages.pdf", many, many_pages, "refused: page cap",
        "101 pages; cap is 100.")
    big_text, big_text_pages = oversized_text()
    add("oversized-text.pdf", big_text, big_text_pages, "refused: text cap",
        "35,000 chars over the 32,000-char document cap; raw size under cap.")
    big_raw, big_raw_pages = oversized_raw_pdf()
    add("oversized-raw.pdf", big_raw, big_raw_pages, "refused: raw cap",
        "Valid PDF with an unreferenced 1.1 MB stream; raw size over 1 MiB.")
    bomb, _ = decompression_bomb()
    add("decompression-bomb.pdf", bomb, [], "refused: extraction bound",
        "One FlateDecode stream expands to ~20 MB of text; spike must show bounded handling.")
    add("malformed-truncated.pdf", normal[: int(len(normal) * 0.6)], [], "refused: parse error",
        "Normal document cut at 60%.")
    bad_header = normal.replace(b"%PDF-1.4", b"XXXX-1.4", 1)
    add("malformed-header.pdf", bad_header, [], "refused: parse error",
        "Bad %PDF magic. NOTE: poppler recovers this input (warning + "
        "text); the spike must confirm lopdf refuses it, else harden "
        "the fixture (corrupt the first object too) before the slice lands.")
    # Zero the xref entry offsets: object lookups can no longer resolve.
    bad_xref = bytearray(normal)
    start = normal.find(b"xref\n")
    end = normal.find(b"trailer", start)
    segment = bytes(bad_xref[start:end])
    lines = segment.split(b"\n")
    out_lines = []
    for line in lines:
        if line.endswith(b" n "):
            out_lines.append(b"0000000000 00000 n ")
        else:
            out_lines.append(line)
    bad_xref[start:end] = b"\n".join(out_lines)
    # Corrupt the trailer's /Root as well: even a parser that rebuilds
    # the xref by scanning must not open a document whose root is gone.
    root_at = bad_xref.find(b"/Root ")
    assert root_at != -1
    num_start = root_at + len(b"/Root ")
    num_end = num_start
    while bad_xref[num_end:num_end + 1].isdigit():
        num_end += 1
    bad_xref[num_start:num_end] = b"99"
    add("malformed-xref.pdf", bytes(bad_xref), [], "refused: parse error",
        "xref offsets zeroed; spike confirms the parser does not silently repair this input.")
    enc, enc_pages = encrypted_pdf()
    add("encrypted-user.pdf", enc, enc_pages, "refused: encrypted",
        "R=2 RC4, user password 'secret'; no password entry in this slice. "
        "Expected text shown only for the pdftotext -upw cross-check.")
    # --- module fixtures (adversarial compressed structures) ---
    os_ok, os_ok_pages = object_stream_ok()
    add("object-stream-ok.pdf", os_ok, os_ok_pages, "imported",
        "PDF 1.5: font + /Pages tree inside a compressed object stream; "
        "xref is a stream. Must import (object stream decodes to ~300 B).")
    os_bomb, _ = object_stream_bomb()
    add("object-stream-bomb.pdf", os_bomb, [], "refused: extraction bound",
        "Unreferenced /ObjStm expanding to ~20 MB; lopdf decompresses "
        "every ObjStm at load, so the pre-load scan must refuse it first.")
    xs_ok, xs_ok_pages = xref_stream_ok()
    add("xref-stream-ok.pdf", xs_ok, xs_ok_pages, "imported",
        "PDF 1.5 with a cross-reference stream, all objects normal. "
        "Must import.")
    xs_bomb, _ = xref_stream_bomb()
    add("xref-stream-bomb.pdf", xs_bomb, [], "refused: extraction bound",
        "Xref stream decoding to ~21 MB (3,000,000 entries x 7 B). "
        "Pre-load scan must refuse it before lopdf parses the xref.")
    cf_ok, cf_ok_pages = chained_filter_ok()
    add("chained-filter-ok.pdf", cf_ok, cf_ok_pages, "imported",
        "Content stream /Filter [/ASCII85Decode /FlateDecode]. Must import.")
    cf_bomb, _ = chained_filter_bomb()
    add("chained-filter-bomb.pdf", cf_bomb, [], "refused: extraction bound",
        "Content stream /Filter [/FlateDecode /FlateDecode] expanding to "
        "~20 MB; the cumulative budget must count both stages.")
    lzw, lzw_pages = lzw_ok()
    add("lzw-ok.pdf", lzw, lzw_pages, "imported",
        "Content stream /Filter /LZWDecode (TIFF early-change). Must import; "
        "validates the generator's LZW encoder against weezl.")
    lzwb, _ = lzw_bomb()
    add("lzw-bomb.pdf", lzwb, [], "refused: extraction bound",
        "LZW content stream expanding to ~20 MB; the capped weezl path "
        "must refuse it.")
    nf_ok, nf_ok_pages = nested_form_ok()
    add("nested-form-ok.pdf", nf_ok, nf_ok_pages, "imported",
        "Page -> Form /Fm1 -> Form /Fm2 -> text. Must import; the "
        "pre-scan walks forms recursively.")
    nf_bomb, _ = nested_form_bomb()
    add("nested-form-bomb.pdf", nf_bomb, [], "refused: extraction bound",
        "Bomb inside the second-level form /Fm2; a one-level form walk "
        "would miss it.")
    # --- repair fixtures (pre-scan vs parser gaps) ---
    ir_ok, ir_ok_pages = indirect_resources_ok()
    add("indirect-resources-ok.pdf", ir_ok, ir_ok_pages, "imported",
        "Page /Resources is an indirect object holding a small Form. "
        "Must import; the pre-scan must resolve the indirect dict.")
    ir_bomb, _ = indirect_resources_bomb()
    add("indirect-resources-bomb.pdf", ir_bomb, [], "refused: extraction bound",
        "Bomb Form reachable only through an indirect /Resources. "
        "The old pre-scan never decoded it.")
    inh_ok, inh_ok_pages = inherited_resources_ok()
    add("inherited-resources-ok.pdf", inh_ok, inh_ok_pages, "imported",
        "Page has no /Resources; the /Pages node carries them (font + "
        "Form). Must import; the pre-scan must walk up the page tree.")
    inh_bomb, _ = inherited_resources_bomb()
    add("inherited-resources-bomb.pdf", inh_bomb, [], "refused: extraction bound",
        "Bomb Form reachable only through inherited /Resources. "
        "The old pre-scan never decoded it.")
    tc_ok, tc_ok_pages = tounicode_compressed_ok()
    add("tounicode-compressed-ok.pdf", tc_ok, tc_ok_pages, "imported",
        "Compressed ToUnicode CMap, 256 bfchar entries (~3 KB decoded). "
        "Must import; the pre-scan must charge the CMap.")
    tc_bomb, _ = tounicode_compressed_bomb()
    add("tounicode-compressed-bomb.pdf", tc_bomb, [], "refused: extraction bound",
        "Compressed ToUnicode CMap, 1,700,000 bfchar entries (~20 MB "
        "decoded, a few KB raw). The old pre-scan never measured CMaps.")
    rf_ok, rf_ok_pages = repeated_form_ok()
    add("repeated-form-ok.pdf", rf_ok, rf_ok_pages, "imported",
        "The same small Form invoked once on each of two pages. Must "
        "import; the pre-scan must charge it once per page.")
    rf_bomb, _ = repeated_form_bomb()
    add("repeated-form-bomb.pdf", rf_bomb, [], "refused: extraction bound",
        "One Form (~1 MB decoded) invoked 20 times on one page: one "
        "decode fits, 20 invocations cost ~20 MB. The old pre-scan "
        "charged it once.")
    sb_ok, sb_ok_pages = stream_boundary_ok()
    add("stream-boundary-ok.pdf", sb_ok, sb_ok_pages, "imported",
        "PDF 1.5 /ObjStm compressed with a stored block carrying the "
        "literal bytes `endstream`, then a zlib block. The loader "
        "trusts /Length; a scan for `endstream` truncates. The "
        "repaired scan uses /Length.")
    with open(os.path.join(OUT, "MANIFEST.json"), "w") as f:
        json.dump(manifest, f, indent=2)
    print("MANIFEST.json written")


if __name__ == "__main__":
    main()
