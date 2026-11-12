import "./app.css";
import { mount } from "svelte";
import App from "./App.svelte";
import { applyPalette, applyTheme, initialPalette, initialTheme } from "./lib/theme";

applyTheme(initialTheme());
applyPalette(initialPalette());

export default mount(App, { target: document.getElementById("app")! });
