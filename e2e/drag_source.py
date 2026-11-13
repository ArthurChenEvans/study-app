# A stand-in file manager: one button you can drag a file from (text/uri-list).
import gi
gi.require_version("Gtk", "3.0")
from gi.repository import Gtk, Gdk
window = Gtk.Window(title="files")
button = Gtk.Button(label="red.png")
button.set_size_request(200, 200)
window.add(button)
button.drag_source_set(Gdk.ModifierType.BUTTON1_MASK, [], Gdk.DragAction.COPY)
button.drag_source_add_uri_targets()
button.connect("drag-data-get", lambda w, ctx, data, info, time: data.set_uris(["file://" + __import__("sys").argv[1] + ""]))
window.connect("destroy", Gtk.main_quit)
window.move(1700, 100)
window.show_all()
Gtk.main()
