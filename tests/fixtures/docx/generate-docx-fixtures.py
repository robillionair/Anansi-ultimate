#!/usr/bin/env python3
"""Generate deterministic DOCX fixtures for the M13 DOCX ingestion spike.

python-docx 1.1.0 builds the base documents; targeted lxml surgery
adds field codes, track changes, page breaks and altChunk; byte-level
surgery builds the corrupt / bomb / encrypted fixtures. Every fixture
is repacked deterministically (fixed ZIP timestamps 1980-01-01, fixed
docProps dates) so re-running the generator reproduces the same bytes
— except the LibreOffice fixture, whose timestamps come from LO and
whose SHA is recorded at generation time.

Run from the repo root:

    python3 tests/fixtures/docx/generate-docx-fixtures.py

Writes the fixtures plus MANIFEST.json (SHA-256, bytes, expected body
+ sections, expected outcome) into tests/fixtures/docx/.

Body model (mirrors the spike): body lines joined by "\\n"; headings
as "# "/"## "/"### " lines (Title -> "# "). Sections split the lines
at heading lines: a leading section {heading: null, level: 0} exists
only when at least one line precedes the first heading; each heading
starts a section {heading, level}. A section's text is its lines
joined by "\\n"; startChar is its char offset in the body (the single
"\\n" separating two sections belongs to neither section's text).
"""

import hashlib
import io
import json
import os
import re
import shutil
import subprocess
import sys
import zipfile

from docx import Document
from docx.oxml.shared import OxmlElement, qn
from docx.opc.constants import RELATIONSHIP_TYPE as RT
from lxml import etree

OUT = os.path.dirname(os.path.abspath(__file__))
FIXED_DATE = (1980, 1, 1, 0, 0, 0)
FIXED_DT = "1980-01-01T00:00:00Z"

W = "http://schemas.openxmlformats.org/wordprocessingml/2006/main"
R = "http://schemas.openxmlformats.org/officeDocument/2006/relationships"
NS = {"w": W, "r": R}

MAX_RAW = 1 << 20
MAX_TEXT = 32_000


# ---------------------------------------------------------------------------
# Deterministic repacking

def normalize_core_xml(data: bytes) -> bytes:
    text = data.decode("utf-8")
    text = re.sub(r"(<dcterms:created[^>]*>)[^<]*(</dcterms:created>)",
                  r"\g<1>" + FIXED_DT + r"\g<2>", text)
    text = re.sub(r"(<dcterms:modified[^>]*>)[^<]*(</dcterms:modified>)",
                  r"\g<1>" + FIXED_DT + r"\g<2>", text)
    return text.encode("utf-8")


def repack(data: bytes, mutate=None, extra=None, remove=(), compress_default=zipfile.ZIP_DEFLATED):
    """Repack a docx deterministically. mutate: {name: new_bytes or fn}.
    extra: {name: (bytes, compress_type)}. remove: names to drop."""
    zin = zipfile.ZipFile(io.BytesIO(data))
    entries = []  # (name, bytes, compress_type)
    for info in zin.infolist():
        if info.filename in remove:
            continue
        content = zin.read(info.filename)
        if info.filename == "docProps/core.xml":
            content = normalize_core_xml(content)
        if mutate and info.filename in mutate:
            value = mutate[info.filename]
            content = value(content) if callable(value) else value
        entries.append((info.filename, content, info.compress_type))
    if extra:
        for name, (content, ctype) in extra.items():
            entries.append((name, content, ctype))
    buf = io.BytesIO()
    zout = zipfile.ZipFile(buf, "w", zipfile.ZIP_DEFLATED)
    for name, content, ctype in entries:
        info = zipfile.ZipInfo(name, date_time=FIXED_DATE)
        info.compress_type = ctype
        info.external_attr = 0o600 << 16
        zout.writestr(info, content)
    zout.close()
    return buf.getvalue()


def save_doc(doc) -> bytes:
    buf = io.BytesIO()
    doc.save(buf)
    return buf.getvalue()


def document_xml(data: bytes) -> bytes:
    zin = zipfile.ZipFile(io.BytesIO(data))
    return zin.read("word/document.xml")


def parse_xml(data: bytes):
    return etree.fromstring(data)


def serialize_xml(root) -> bytes:
    return etree.tostring(root, xml_declaration=True, encoding="UTF-8", standalone=True)


def body_of(root):
    return root.find("w:body", NS)


def append_paragraph_before_sectpr(root, para_xml: str):
    """Insert a raw <w:p> ... </w:p> element just before <w:sectPr>."""
    body = body_of(root)
    para = etree.fromstring(para_xml.encode("utf-8"))
    sectpr = body.find("w:sectPr", NS)
    if sectpr is not None:
        sectpr.addprevious(para)
    else:
        body.append(para)


def add_hyperlink(paragraph, url, text):
    """Standard python-docx hyperlink recipe (external relationship)."""
    part = paragraph.part
    r_id = part.relate_to(url, RT.HYPERLINK, is_external=True)
    hyperlink = OxmlElement("w:hyperlink")
    hyperlink.set(qn("r:id"), r_id)
    run = OxmlElement("w:r")
    run.append(OxmlElement("w:rPr"))
    t = OxmlElement("w:t")
    t.text = text
    run.append(t)
    hyperlink.append(run)
    paragraph._p.append(hyperlink)
    return hyperlink


# ---------------------------------------------------------------------------
# Expected body / sections (same rule as the spike)

def body_and_sections(lines):
    """lines: [(text, level_or_None)]. Returns (body, sections)."""
    body_lines = []
    for text, level in lines:
        if level is None:
            body_lines.append(text)
        else:
            body_lines.append("#" * level + " " + text)
    body = "\n".join(body_lines)
    groups = []
    cur = {"heading": None, "level": 0, "lines": []}
    for text, level in lines:
        if level is not None:
            if cur["lines"]:
                groups.append(cur)
            cur = {"heading": text, "level": level, "lines": [text]}
        else:
            cur["lines"].append(text)
    if cur["lines"]:
        groups.append(cur)
    sections = []
    pos = 0
    for g in groups:
        # Heading lines carry their "# " prefix in the body.
        rendered = []
        for i, t in enumerate(g["lines"]):
            if i == 0 and g["heading"] is not None:
                rendered.append("#" * g["level"] + " " + t)
            else:
                rendered.append(t)
        text = "\n".join(rendered)
        sections.append({"heading": g["heading"], "level": g["level"],
                         "startChar": pos, "chars": len(text)})
        pos += len(text) + 1
    return body, sections


MANIFEST = {}


def record(name, data, lines, outcome, notes):
    body, sections = body_and_sections(lines) if lines is not None else (None, None)
    with open(os.path.join(OUT, name), "wb") as f:
        f.write(data)
    MANIFEST[name] = {
        "sha256": hashlib.sha256(data).hexdigest(),
        "bytes": len(data),
        "expected_body": body,
        "expected_sections": sections,
        "expected_outcome": outcome,
        "notes": notes,
    }


# ---------------------------------------------------------------------------
# Fixtures

def _normal_doc_and_lines():
    """The normal document (bytes + expected lines), shared by
    fixture_normal and fixture_outline_lvl."""
    doc = Document()
    doc.add_heading("Harbor Ledger", level=0)  # Title style
    doc.add_heading("Customs", level=1)
    doc.add_paragraph("The bramblewick shipment cleared customs on Tuesday.")
    doc.add_heading("Manifest", level=2)
    doc.add_paragraph("Three crates of copper fittings, two of rope.")
    table = doc.add_table(rows=2, cols=2)
    table.cell(0, 0).text = "Copper"
    table.cell(0, 1).text = "3 crates"
    table.cell(1, 0).text = "Rope"
    table.cell(1, 1).text = "2 coils"
    doc.add_heading("Notes", level=3)
    p = doc.add_paragraph()
    add_hyperlink(p, "https://example.invalid/annex", "See the annex")
    doc.add_paragraph("First item", style="List Bullet")
    doc.add_paragraph("Second item", style="List Bullet")
    doc.add_paragraph("Totals: 41 crates, 12 pallets, 1 lighthouse keeper.")
    data = save_doc(doc)
    # Mid-paragraph page break, via lxml surgery (inserted before
    # sectPr, i.e. after the last paragraph, "Totals ...").
    root = parse_xml(document_xml(data))
    para_xml = (
        '<w:p xmlns:w="%s">'
        '<w:r><w:t>Before the break</w:t></w:r>'
        '<w:r><w:br w:type="page"/></w:r>'
        '<w:r><w:t>After the break</w:t></w:r>'
        "</w:p>" % W
    )
    append_paragraph_before_sectpr(root, para_xml)
    data = repack(data, mutate={"word/document.xml": serialize_xml(root)})
    lines = [
        ("Harbor Ledger", 1),
        ("Customs", 1),
        ("The bramblewick shipment cleared customs on Tuesday.", None),
        ("Manifest", 2),
        ("Three crates of copper fittings, two of rope.", None),
        ("Copper", None), ("3 crates", None), ("Rope", None), ("2 coils", None),
        ("Notes", 3),
        ("See the annex", None),
        ("First item", None), ("Second item", None),
        ("Totals: 41 crates, 12 pallets, 1 lighthouse keeper.", None),
        ("Before the break", None), ("After the break", None),
    ]
    return data, lines


def fixture_normal():
    data, lines = _normal_doc_and_lines()
    record("normal.docx", data, lines, "imported",
           "Title + H1/H2/H3, paragraphs, 2x2 table (cells as lines), "
           "external hyperlink (display text only), list (no number "
           "expansion), mid-paragraph page break (two lines, no form feed).")


def fixture_outline_lvl():
    # Same document as normal, but every heading carries ONLY
    # w:outlineLvl (no w:pStyle): the design's heading fallback path.
    # Title/Heading1 -> outlineLvl 0, Heading2 -> 1, Heading3 -> 2.
    data, lines = _normal_doc_and_lines()
    xml = document_xml(data).decode("utf-8")
    for style, lvl in [("Title", 0), ("Heading1", 0),
                       ("Heading2", 1), ("Heading3", 2)]:
        old = '<w:pStyle w:val="%s"/>' % style
        assert xml.count(old) == 1, (style, xml.count(old))
        xml = xml.replace(old, '<w:outlineLvl w:val="%d"/>' % lvl)
    assert xml.count("w:outlineLvl") == 4
    for style in ("Title", "Heading1", "Heading2", "Heading3"):
        assert 'w:val="%s"' % style not in xml, style
    data = repack(data, mutate={"word/document.xml": xml.encode("utf-8")})
    record("outline-lvl.docx", data, lines, "imported",
           "Same body as normal.docx, but all four headings carry only "
           "w:outlineLvl (0/0/1/2) and no w:pStyle: exercises the "
           "outlineLvl heading fallback. List Bullet paragraphs keep "
           "their pStyle.")




def fixture_unicode_headings():
    doc = Document()
    doc.add_heading("海关总署", level=1)
    doc.add_paragraph("中文段落：货物已清关。")
    doc.add_heading("Café ☕", level=2)
    doc.add_paragraph("Un café crème, s'il vous plaît. ☕")
    doc.add_heading("naïve façade — ünïcödé", level=3)
    doc.add_paragraph("Æsop's fables — naïve façade. 😀")
    data = repack(save_doc(doc))
    lines = [
        ("海关总署", 1),
        ("中文段落：货物已清关。", None),
        ("Café ☕", 2),
        ("Un café crème, s'il vous plaît. ☕", None),
        ("naïve façade — ünïcödé", 3),
        ("Æsop's fables — naïve façade. 😀", None),
    ]
    record("unicode-headings.docx", data, lines, "imported",
           "CJK, emoji and accented headings + paragraphs; exact code points.")


def _field_para(field_code, result_text):
    return (
        '<w:p xmlns:w="%s" xmlns:r="%s">'
        '<w:r><w:fldChar w:fldCharType="begin"/></w:r>'
        '<w:r><w:instrText xml:space="preserve"> %s </w:instrText></w:r>'
        '<w:r><w:fldChar w:fldCharType="separate"/></w:r>'
        '<w:r><w:t>%s</w:t></w:r>'
        '<w:r><w:fldChar w:fldCharType="end"/></w:r>'
        "</w:p>" % (W, R, field_code, result_text)
    )


def fixture_field_codes():
    doc = Document()
    doc.add_paragraph("Placeholder one.")
    doc.add_paragraph("Placeholder two.")
    doc.add_paragraph("After the fields.")
    data = save_doc(doc)
    root = parse_xml(document_xml(data))
    body = body_of(root)
    paras = body.findall("w:p", NS)
    # Replace the first two paragraphs with field paragraphs.
    for para, xml in [
        (paras[0], _field_para('HYPERLINK "https://example.invalid/annex"', "Annex link")),
        (paras[1], _field_para(r"PAGEREF _Toc123 \h", "3")),
    ]:
        new_para = etree.fromstring(xml.encode("utf-8"))
        para.addprevious(new_para)
        body.remove(para)
    data = repack(data, mutate={"word/document.xml": serialize_xml(root)})
    lines = [
        ("Annex link", None),
        ("3", None),
        ("After the fields.", None),
    ]
    record("field-codes.docx", data, lines, "imported",
           "HYPERLINK + PAGEREF fields: instrText excluded, field results "
           "included, URLs never in the body.")


def fixture_track_changes():
    doc = Document()
    doc.add_paragraph("Placeholder one.")
    doc.add_paragraph("Placeholder two.")
    doc.add_paragraph("Final text.")
    data = save_doc(doc)
    root = parse_xml(document_xml(data))
    body = body_of(root)
    paras = body.findall("w:p", NS)
    ins_xml = (
        '<w:p xmlns:w="%s">'
        '<w:ins w:id="1" w:author="Muse" w:date="2026-11-09T00:00:00Z">'
        '<w:r><w:t>Inserted text.</w:t></w:r></w:ins>'
        '<w:r><w:t xml:space="preserve"> Kept text.</w:t></w:r>'
        "</w:p>" % W
    )
    del_xml = (
        '<w:p xmlns:w="%s">'
        '<w:del w:id="2" w:author="Muse" w:date="2026-11-09T00:00:00Z">'
        '<w:r><w:delText>Deleted text.</w:delText></w:r></w:del>'
        '<w:del w:id="3" w:author="Muse" w:date="2026-11-09T00:00:00Z">'
        '<w:r><w:t>Deleted w:t text.</w:t></w:r></w:del>'
        "</w:p>" % W
    )
    for para, xml in [(paras[0], ins_xml), (paras[1], del_xml)]:
        new_para = etree.fromstring(xml.encode("utf-8"))
        para.addprevious(new_para)
        body.remove(para)
    data = repack(data, mutate={"word/document.xml": serialize_xml(root)})
    lines = [
        ("Inserted text. Kept text.", None),
        ("Final text.", None),
    ]
    record("track-changes.docx", data, lines, "imported",
           "w:ins included, w:del skipped (both w:delText and w:t inside "
           "w:del); deleted paragraph yields no line.")


def fixture_empty():
    doc = Document()
    data = repack(save_doc(doc))
    record("empty.docx", data, [], "ok: empty",
           "Valid DOCX, no paragraphs; extraction yields 0 chars. The "
           "worker maps empty text to a refusal (as for PDF empty-text).")


def fixture_whitespace_only():
    doc = Document()
    doc.add_paragraph("   ")
    doc.add_paragraph("\t")
    doc.add_paragraph("")
    doc.add_paragraph("  \t  ")
    data = repack(save_doc(doc))
    record("whitespace-only.docx", data, [], "ok: empty",
           "Whitespace-only paragraphs are skipped; 0 chars extracted.")


def fixture_external_hyperlink():
    doc = Document()
    p = doc.add_paragraph()
    add_hyperlink(p, "https://example.invalid/annex", "External annex link")
    doc.add_paragraph("Plain paragraph after the link.")
    data = repack(save_doc(doc))
    lines = [
        ("External annex link", None),
        ("Plain paragraph after the link.", None),
    ]
    record("external-hyperlink.docx", data, lines, "imported",
           "Hyperlink with TargetMode=External: display text imported, "
           "URL not in the body, nothing fetched.")


def fixture_altchunk():
    doc = Document()
    doc.add_paragraph("Before the chunk.")
    data = save_doc(doc)
    root = parse_xml(document_xml(data))
    body = body_of(root)
    alt = etree.fromstring(
        ('<w:altChunk xmlns:w="%s" xmlns:r="%s" r:id="rIdAlt"/>' % (W, R)).encode("utf-8"))
    sectpr = body.find("w:sectPr", NS)
    if sectpr is not None:
        sectpr.addprevious(alt)
    else:
        body.append(alt)
    data = repack(data, mutate={"word/document.xml": serialize_xml(root)})
    record("altchunk.docx", data, None, "refused: altChunk",
           "w:altChunk present: importing would publish a partial document.")


def fixture_doctype():
    # A DOCTYPE with an entity declaration inside word/document.xml:
    # the pre-scan must refuse it before any XML parsing (no DTD, no
    # entity expansion, no billion-laughs class). String surgery on
    # the serialized XML: the declaration is inserted directly after
    # the XML declaration, so the document is otherwise valid.
    doc = Document()
    doc.add_paragraph("Harmless text behind a DOCTYPE.")
    data = save_doc(doc)
    xml = document_xml(data).decode("utf-8")
    decl = "<?xml version='1.0' encoding='UTF-8' standalone='yes'?>"
    assert xml.startswith(decl), xml[:80]
    xml = xml.replace(
        decl,
        decl + '<!DOCTYPE w:document [ <!ENTITY x "laughs"> ]>',
        1)
    data = repack(data, mutate={"word/document.xml": xml.encode("utf-8")})
    record("doctype.docx", data, None, "refused: DOCTYPE",
           "word/document.xml carries <!DOCTYPE with an entity "
           "declaration: refused by the pre-scan before parsing.")



def _base_bytes():
    doc = Document()
    doc.add_heading("Harbor Ledger", level=1)
    doc.add_paragraph("The bramblewick shipment cleared customs on Tuesday.")
    return repack(save_doc(doc))


def fixture_corrupt_truncated():
    data = _base_bytes()
    cut = data[: int(len(data) * 0.6)]
    record("corrupt-truncated-zip.docx", cut, None, "refused: parse error",
           "Valid document cut at 60% of its bytes.")


def fixture_corrupt_not_a_zip():
    data = b"This is not a zip archive, just plain text.\n"
    record("corrupt-not-a-zip.docx", data, None, "refused: parse error",
           "Not a ZIP archive at all.")


def fixture_corrupt_bad_xml():
    data = _base_bytes()
    bad = (b'<w:document xmlns:w="http://schemas.openxmlformats.org/'
           b'wordprocessingml/2006/main"><w:body><w:p><w:t>Unclosed')
    data = repack(data, mutate={"word/document.xml": bad})
    record("corrupt-bad-xml.docx", data, None, "refused: parse error",
           "word/document.xml is malformed XML (unclosed elements).")


def fixture_corrupt_missing_document():
    data = _base_bytes()
    data = repack(data, remove=("word/document.xml",))
    record("corrupt-missing-document.docx", data, None,
           "refused: missing word/document.xml",
           "Archive without word/document.xml.")


def fixture_corrupt_missing_content_types():
    data = _base_bytes()
    data = repack(data, remove=("[Content_Types].xml",))
    record("corrupt-missing-content-types.docx", data, None,
           "refused: missing [Content_Types].xml",
           "Archive without [Content_Types].xml (malformed OPC package).")


def _flip_byte_in_document_data(data: bytes) -> bytes:
    """Flip one byte inside the compressed word/document.xml stream."""
    pos = data.find(b"word/document.xml")
    assert pos > 0
    # Local file header starts 30 bytes before the filename.
    header = pos - 30
    assert data[header:header + 4] == b"PK\x03\x04"
    name_len = int.from_bytes(data[header + 26:header + 28], "little")
    extra_len = int.from_bytes(data[header + 28:header + 30], "little")
    data_start = header + 30 + name_len + extra_len
    out = bytearray(data)
    out[data_start + 10] ^= 0xFF
    return bytes(out)


def fixture_corrupt_bad_crc():
    data = _flip_byte_in_document_data(_base_bytes())
    record("corrupt-bad-crc.docx", data, None, "refused: parse error",
           "One flipped byte inside the compressed word/document.xml "
           "stream: decompression or CRC must fail.")


def fixture_oversized_raw():
    data = _base_bytes()
    pad = bytes(1_100_000)  # zeros, stored uncompressed
    data = repack(data, extra={"word/media/pad.bin": (pad, zipfile.ZIP_STORED)})
    assert len(data) > MAX_RAW, len(data)
    record("oversized-raw.docx", data, None, "refused: raw cap",
           "Valid document + 1.1 MB stored pad entry; raw size over 1 MiB.")


def fixture_oversized_text():
    doc = Document()
    for _ in range(5):
        doc.add_paragraph("b" * 7000)
    data = repack(save_doc(doc))
    record("oversized-text.docx", data, None, "refused: text cap",
           "35,000 chars over the 32,000-char document cap; raw size under cap.")


def _patch_declared_size(data: bytes, name: str, new_size: int) -> bytes:
    """Patch the uncompressed-size field for `name` in both the local
    header and the central directory. Used for the lying-size bomb."""
    out = bytearray(data)
    # Central directory: find EOCD, walk entries.
    eocd = out.rfind(b"PK\x05\x06")
    assert eocd > 0
    cd_offset = int.from_bytes(out[eocd + 16:eocd + 20], "little")
    cd_entries = int.from_bytes(out[eocd + 10:eocd + 12], "little")
    pos = cd_offset
    patched_cd = False
    for _ in range(cd_entries):
        assert out[pos:pos + 4] == b"PK\x01\x02", pos
        name_len = int.from_bytes(out[pos + 28:pos + 30], "little")
        extra_len = int.from_bytes(out[pos + 30:pos + 32], "little")
        comment_len = int.from_bytes(out[pos + 32:pos + 34], "little")
        entry_name = bytes(out[pos + 46:pos + 46 + name_len]).decode("utf-8")
        if entry_name == name:
            out[pos + 24:pos + 28] = new_size.to_bytes(4, "little")
            patched_cd = True
        pos += 46 + name_len + extra_len + comment_len
    assert patched_cd, name
    # Local header.
    lpos = out.find(b"PK\x03\x04")
    patched_local = False
    while lpos >= 0:
        name_len = int.from_bytes(out[lpos + 26:lpos + 28], "little")
        entry_name = bytes(out[lpos + 30:lpos + 30 + name_len]).decode("utf-8")
        if entry_name == name:
            out[lpos + 18:lpos + 22] = new_size.to_bytes(4, "little")
            patched_local = True
            break
        lpos = out.find(b"PK\x03\x04", lpos + 4)
    assert patched_local, name
    return bytes(out)


def fixture_zip_bomb():
    # One paragraph of 20,000,000 'A' chars: ~20 MB inflated, ~20 KB raw.
    doc = Document()
    doc.add_paragraph("A" * 20_000_000)
    data = repack(save_doc(doc))
    raw_before = len(data)
    # Lie about the declared size so the 8 MiB single-entry pre-check
    # does NOT fire: the refusal must happen during inflation. (The
    # honest-size variant is covered by the spike's in-memory worstbomb
    # mode, which expects the declared-size refusal.)
    data = _patch_declared_size(data, "word/document.xml", 1000)
    # Spike finding: the zip crate itself refuses a lying declared size
    # during inflation ("File is larger than its declared uncompressed
    # size"), so the capped reader is the second layer, not the first.
    # The capped reader's own mechanism is proven directly by the
    # spike's worstbomb CappedReader unit check.
    record("zip-bomb.docx", data, None, "refused: declared uncompressed size",
           "word/document.xml inflates to ~20 MB from %d raw bytes; "
           "declared size patched to 1000. The zip crate refuses the "
           "lying size during inflation; the capped reader is the "
           "second layer." % raw_before)


def fixture_many_entries():
    data = _base_bytes()
    extra = {}
    for i in range(130):
        extra["word/media/dummy%03d.bin" % i] = (b"0123456789", zipfile.ZIP_DEFLATED)
    data = repack(data, extra=extra)
    record("many-entries.docx", data, None, "refused: entry cap",
           "Valid document + 130 dummy entries (> 128 total).")


def _flip_gp_flag(data: bytes, name: str) -> bytes:
    """Set general-purpose bit 0 (encrypted) on `name` in both the
    local header and the central directory."""
    out = bytearray(data)
    eocd = out.rfind(b"PK\x05\x06")
    assert eocd > 0
    cd_offset = int.from_bytes(out[eocd + 16:eocd + 20], "little")
    cd_entries = int.from_bytes(out[eocd + 10:eocd + 12], "little")
    pos = cd_offset
    patched_cd = False
    for _ in range(cd_entries):
        assert out[pos:pos + 4] == b"PK\x01\x02", pos
        name_len = int.from_bytes(out[pos + 28:pos + 30], "little")
        extra_len = int.from_bytes(out[pos + 30:pos + 32], "little")
        comment_len = int.from_bytes(out[pos + 32:pos + 34], "little")
        entry_name = bytes(out[pos + 46:pos + 46 + name_len]).decode("utf-8")
        if entry_name == name:
            flags = int.from_bytes(out[pos + 8:pos + 10], "little") | 0x1
            out[pos + 8:pos + 10] = flags.to_bytes(2, "little")
            patched_cd = True
        pos += 46 + name_len + extra_len + comment_len
    assert patched_cd, name
    lpos = out.find(b"PK\x03\x04")
    patched_local = False
    while lpos >= 0:
        name_len = int.from_bytes(out[lpos + 26:lpos + 28], "little")
        entry_name = bytes(out[lpos + 30:lpos + 30 + name_len]).decode("utf-8")
        if entry_name == name:
            flags = int.from_bytes(out[lpos + 6:lpos + 8], "little") | 0x1
            out[lpos + 6:lpos + 8] = flags.to_bytes(2, "little")
            patched_local = True
            break
        lpos = out.find(b"PK\x03\x04", lpos + 4)
    assert patched_local, name
    return bytes(out)


def fixture_encrypted_entry():
    data = _flip_gp_flag(_base_bytes(), "word/document.xml")
    record("encrypted-entry.docx", data, None, "refused: encrypted entry",
           "General-purpose bit 0 set on word/document.xml (flag-only "
           "synthetic; content untouched). The parser must refuse, never "
           "attempt decryption.")


def fixture_macro():
    data = _base_bytes()
    data = repack(data, extra={"word/vbaProject.bin": (b"VBA!", zipfile.ZIP_DEFLATED)})
    record("macro.docx", data, None, "refused: macro",
           "word/vbaProject.bin present (active content).")


def fixture_libreoffice():
    src = os.path.join(OUT, "normal.docx")
    tmp = os.path.join(OUT, ".lo-tmp")
    os.makedirs(tmp, exist_ok=True)
    try:
        subprocess.run(
            ["soffice", "--headless", "--convert-to", "docx",
             "--outdir", tmp, src],
            check=True, capture_output=True, text=True, timeout=120)
        converted = os.path.join(tmp, "normal.docx")
        with open(converted, "rb") as f:
            data = f.read()
        # Deterministic repack (LO timestamps in docProps stay as-is;
        # the SHA is recorded at generation time).
        data = repack(data)
        with open(os.path.join(OUT, "libreoffice-realworld.docx"), "wb") as f:
            f.write(data)
        # Expected text: the LO round-trip must preserve normal.docx's text.
        body, sections = body_and_sections([
            ("Harbor Ledger", 1),
            ("Customs", 1),
            ("The bramblewick shipment cleared customs on Tuesday.", None),
            ("Manifest", 2),
            ("Three crates of copper fittings, two of rope.", None),
            ("Copper", None), ("3 crates", None), ("Rope", None), ("2 coils", None),
            ("Notes", 3),
            ("See the annex", None),
            ("First item", None), ("Second item", None),
            ("Totals: 41 crates, 12 pallets, 1 lighthouse keeper.", None),
            ("Before the break", None), ("After the break", None),
        ])
        MANIFEST["libreoffice-realworld.docx"] = {
            "sha256": hashlib.sha256(data).hexdigest(),
            "bytes": len(data),
            "expected_body": body,
            "expected_sections": sections,
            "expected_outcome": "imported",
            "notes": "LibreOffice round-trip of normal.docx (soffice "
                     "--headless --convert-to docx). SHA recorded at "
                     "generation time; LO timestamps are not reproducible, "
                     "so re-running the generator changes the SHA. "
                     "Cross-checked against soffice --convert-to txt.",
        }
    finally:
        shutil.rmtree(tmp, ignore_errors=True)


def main():
    fixture_normal()
    fixture_outline_lvl()
    fixture_unicode_headings()
    fixture_field_codes()
    fixture_track_changes()
    fixture_empty()
    fixture_whitespace_only()
    fixture_external_hyperlink()
    fixture_altchunk()
    fixture_doctype()
    fixture_corrupt_truncated()
    fixture_corrupt_not_a_zip()
    fixture_corrupt_bad_xml()
    fixture_corrupt_missing_document()
    fixture_corrupt_missing_content_types()
    fixture_corrupt_bad_crc()
    fixture_oversized_raw()
    fixture_oversized_text()
    fixture_zip_bomb()
    fixture_many_entries()
    fixture_encrypted_entry()
    fixture_macro()
    fixture_libreoffice()
    with open(os.path.join(OUT, "MANIFEST.json"), "w", encoding="utf-8") as f:
        json.dump({"fixtures": MANIFEST}, f, indent=2, ensure_ascii=False)
        f.write("\n")
    print("wrote %d fixtures" % len(MANIFEST))
    for name in sorted(MANIFEST):
        m = MANIFEST[name]
        print("  %-32s %8d bytes  %s" % (name, m["bytes"], m["expected_outcome"]))


if __name__ == "__main__":
    main()
