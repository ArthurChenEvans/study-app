import "./app.css";
import { mount } from "svelte";
import App from "./App.svelte";
import { applyPalette, applyTheme, followTheme, initialPalette, initialTheme } from "./lib/theme";

applyTheme(initialTheme());
applyPalette(initialPalette());
followTheme();

export default mount(App, { target: document.getElementById("app")! });
