"""Makes the Anki files the import tests read (import-export.feature, R171).

Uses Anki's own library, so the files are exactly what Anki writes. Run it in a venv with
`pip install anki`:  python scripts/make-anki-fixtures.py crates/study-api/tests/fixtures/anki
"""
import os, pathlib, struct, sys, tempfile, zlib
from anki.collection import Collection, ExportAnkiPackageOptions
from anki.decks import DeckId

out = pathlib.Path(sys.argv[1]).resolve()
out.mkdir(parents=True, exist_ok=True)


def png(width, height, rgb):
    raw = b"".join(b"\x00" + bytes(rgb) * width for _ in range(height))
    chunk = lambda kind, data: struct.pack(">I", len(data)) + kind + data + struct.pack(">I", zlib.crc32(kind + data) & 0xFFFFFFFF)
    return b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", struct.pack(">IIBBBBB", width, height, 8, 2, 0, 0, 0)) + chunk(b"IDAT", zlib.compress(raw)) + chunk(b"IEND", b"")


folder = tempfile.mkdtemp()
col = Collection(os.path.join(folder, "collection.anki2"))
col.media.write_data("tlb.png", png(4, 3, (200, 60, 20)))
col.media.write_data("click.mp3", b"ID3 not really audio")

dns = col.decks.id("Networks::DNS")
tls = col.decks.id("Networks::TLS")
misc = col.decks.id("Misc")


def add(model_name, deck, fields, tags=()):
    model = col.models.by_name(model_name)
    note = col.new_note(model)
    for index, value in enumerate(fields):
        note.fields[index] = value
    note.tags = list(tags)
    col.add_note(note, DeckId(deck))
    return note


basic = add("Basic", dns, ["What resolves a name to an <b>IP address</b>?", "DNS<br>Domain Name System"], ["networks"])
add("Basic", dns, ["What caches page table entries?", 'The TLB <img src="tlb.png">'])
add("Basic (and reversed card)", tls, ["TLS", "Transport Layer Security"])
add("Cloze", tls, ["The handshake agrees a {{c1::cipher suite}} and swaps {{c2::keys::what}}.", "From lecture 5"])
add("Basic", misc, ["Sound check [sound:click.mp3]", "It made a click &amp; a beep"])

# A note type Shudhana doesn't have: it comes in as Basic from its first two fields.
vocab = col.models.new("Vocab")
for name in ("Thai", "English", "Notes"):
    col.models.add_field(vocab, col.models.new_field(name))
template = col.models.new_template("Card 1")
template["qfmt"] = "{{Thai}}"
template["afmt"] = "{{FrontSide}}<hr id=answer>{{English}}"
col.models.add_template(vocab, template)
col.models.add(vocab)
add("Vocab", misc, ["จำ", "to remember", "jam"])

# Some progress: review the first card a few times so it has an interval.
card = basic.cards()[0]
card.start_timer()
col.sched.answerCard(card, 3)  # Good
everything = col.decks.id("Networks")
for name, legacy in (("networks-modern.apkg", False), ("networks-legacy.apkg", True)):
    options = ExportAnkiPackageOptions(with_scheduling=True, with_deck_configs=False, with_media=True, legacy=legacy)
    col.export_anki_package(out_path=str(out / name), options=options, limit=None)
col.export_collection_package(str(out / "whole-collection.colpkg"), include_media=True, legacy=False)
col.close()
for path in sorted(out.iterdir()):
    print(path.name, path.stat().st_size)
