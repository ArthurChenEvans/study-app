"""Opens a deck the app exported in Anki itself (import-export.feature, R173).

  ANKI_OUT=/tmp/x.apkg cargo test -p study-api write_an_anki_export -- --ignored
  python scripts/check-anki-export.py /tmp/x.apkg      (in a venv with `pip install anki`)
"""
import os, sys, tempfile
from anki.collection import Collection, ImportAnkiPackageRequest

path = sys.argv[1]
col = Collection(os.path.join(tempfile.mkdtemp(), "collection.anki2"))
col.import_anki_package(ImportAnkiPackageRequest(package_path=path))
decks = sorted(name.name for name in col.decks.all_names_and_ids() if name.name != "Default")
notes = [col.get_note(nid) for nid in col.find_notes("")]
print("decks:", decks)
print("notes:", len(notes), "cards:", col.card_count())
for note in notes:
    print(" ", note.note_type()["name"], "|", " || ".join(field.replace("\n", " ")[:70] for field in note.fields))
media = os.listdir(col.media.dir())
print("media:", media)
assert decks == ["Networks", "Networks::DNS"], decks
assert {note.note_type()["name"] for note in notes} == {"Shudhana Basic", "Shudhana Basic (and reversed card)", "Shudhana Cloze"}
assert col.card_count() == 6, col.card_count()  # basic 1, reversed 2, cloze 2, quiz 1
assert len(media) == 1 and all('src="' + media[0] in note.fields[1] for note in notes if "diagram" in note.fields[1])
# A second deck reuses the same note types rather than adding more.
col.import_anki_package(ImportAnkiPackageRequest(package_path=path))
assert sum(1 for model in col.models.all_names_and_ids() if model.name.startswith("Shudhana")) == 3
print("Anki opened it: OK")
