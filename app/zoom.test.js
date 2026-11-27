// node zoom.test.js — the 3 zoom keys of the app (Ctrl / Cmd + 0, + or =, −), nothing else.
const assert = require("assert");
const { toucheZoom, pourApp } = require("./zoom.js");
const k = (key, o = {}) => ({ key, ctrlKey: false, metaKey: false, altKey: false, ...o });
assert.strictEqual(toucheZoom(k("0", { ctrlKey: true })), "reset");
assert.strictEqual(toucheZoom(k("0", { metaKey: true })), "reset", "Cmd on a Mac");
assert.strictEqual(toucheZoom(k("=", { ctrlKey: true })), "in");
assert.strictEqual(toucheZoom(k("+", { ctrlKey: true })), "in");
assert.strictEqual(toucheZoom(k("-", { ctrlKey: true })), "out");
assert.strictEqual(toucheZoom(k("0")), null, "without Ctrl / Cmd: not a zoom key");
assert.strictEqual(toucheZoom(k("k", { ctrlKey: true })), null, "Ctrl + K stays the app's search");
assert.strictEqual(toucheZoom(k("0", { ctrlKey: true, altKey: true })), null);
assert.strictEqual(pourApp({ tagName: "CANVAS" }), true);
assert.strictEqual(pourApp({ tagName: "INPUT" }), false, "a text field keeps the browser's keys");
assert.strictEqual(pourApp(null), true);
console.log("zoom.js : ok");
