import "./app.css";
import { mount } from "svelte";
import App from "./App.svelte";
import { applyTheme, initialTheme } from "./lib/theme";

applyTheme(initialTheme());

export default mount(App, { target: document.getElementById("app")! });
