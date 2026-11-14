"""Makes the one-big-deck Anki file the tag and suspended-card import tests read (R188).

One deck holding everything, filtered by tags (the Michael Nielsen way), with some cards
suspended. Run it in a venv with `pip install anki`:
  python scripts/make-anki-tag-fixture.py crates/study-api/tests/fixtures/anki
"""
import os, pathlib, sys, tempfile
from anki.collection import Collection, ExportAnkiPackageOptions
from anki.decks import DeckId

out = pathlib.Path(sys.argv[1]).resolve()
out.mkdir(parents=True, exist_ok=True)
col = Collection(os.path.join(tempfile.mkdtemp(), "collection.anki2"))
everything = col.decks.id("Everything")


def add(model_name, fields, tags=()):
    note = col.new_note(col.models.by_name(model_name))
    for index, value in enumerate(fields):
        note.fields[index] = value
    note.tags = list(tags)
    col.add_note(note, DeckId(everything))
    return note


add("Basic", ["2 + 2", "4"], ["Maths::Algebra"])
add("Basic", ["Angles in a triangle", "180 degrees"], ["Maths::Geometry"])
add("Basic", ["What is a group?", "A set with an associative operation, identity and inverses"], ["maths"])
add("Basic", ["จำ", "to remember"], ["thai"])
sleep = add("Basic", ["นอน", "to sleep"], ["thai"])
add("Basic", ["Untagged note", "No tags at all"])
both = add("Basic (and reversed card)", ["Integral of 1/x", "ln|x| + C"], ["Maths::Calculus", "thai"])

# Suspended: the whole of one note, and just the reverse card of another.
col.sched.suspend_cards([card.id for card in sleep.cards()])
col.sched.suspend_cards([both.cards()[1].id])

options = ExportAnkiPackageOptions(with_scheduling=True, with_deck_configs=False, with_media=False, legacy=False)
col.export_anki_package(out_path=str(out / "one-deck-tagged.apkg"), options=options, limit=None)
col.close()
print((out / "one-deck-tagged.apkg").stat().st_size)
