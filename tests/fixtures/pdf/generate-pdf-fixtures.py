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
    with open(os.path.join(OUT, "MANIFEST.json"), "w") as f:
        json.dump(manifest, f, indent=2)
    print("MANIFEST.json written")


if __name__ == "__main__":
    main()
