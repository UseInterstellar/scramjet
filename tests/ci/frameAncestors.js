/**
 * @fileoverview
 * Regression test for `ScramjetClient#frame` when a proxied frame is nested inside pages that are not running a
 * Scramjet client (e.g. a host page that wraps `ScramjetFrame` iframes in its own shell iframes).
 * The getter walks every ancestor window; upstream assumed each one had a client and threw
 * "Cannot read properties of undefined (reading 'descriptors')" during client construction.
 *
 * The real getter source is pulled out of src/client/client.ts and run against fake window chains,
 * so this fails if the guard is removed. Real-browser nesting still needs a manual check.
 */

import test from "ava";
import { readFileSync } from "node:fs";

// client.ts may be checked out with CRLF line endings
const source = readFileSync("src/client/client.ts", "utf8")
	.split(String.fromCharCode(13))
	.join("");
const start = source.indexOf("get frame(): ScramjetFrame | null {");
const end = source.indexOf("\n\t}\n", start);
if (start < 0 || end < 0) throw new Error("could not locate ScramjetClient#frame");

// strip the TS return type, the rest of the getter is plain JS
const body = source
	.slice(start, end + 3)
	.replace("get frame(): ScramjetFrame | null {", "{");

const SCRAMJETFRAME = Symbol("frame");
const SCRAMJETCLIENT = Symbol("client");
const getFrame = new Function(
	"iswindow",
	"SCRAMJETFRAME",
	"SCRAMJETCLIENT",
	`return function frame() ${body}`
)(true, SCRAMJETFRAME, SCRAMJETCLIENT);

/** a window whose frameElement is `frameElement`, optionally running a scramjet client */
function makeWindow(parent, frameElement, hasClient) {
	const win = { frameElement };
	win.window = win;
	win.parent = parent ?? win;
	if (hasClient) {
		win[SCRAMJETCLIENT] = {
			descriptors: {
				get: (_name, w) => w.frameElement,
			},
		};
	}
	return win;
}

/** evaluates the real getter with a client for `win` as `this` */
function frameOf(win) {
	return getFrame.call({
		global: win,
		descriptors: win[SCRAMJETCLIENT].descriptors,
	});
}

test("top level window has no frame", (t) => {
	const top = makeWindow(null, null, true);
	t.is(frameOf(top), null);
});

test("direct ScramjetFrame iframe returns its frame", (t) => {
	const sframe = { name: "sf" };
	const top = makeWindow(null, null, true);
	const child = makeWindow(top, { [SCRAMJETFRAME]: sframe }, true);
	t.is(frameOf(child), sframe);
});

test("subframe finds the ScramjetFrame of an ancestor client", (t) => {
	const sframe = { name: "sf" };
	const top = makeWindow(null, null, true);
	const mid = makeWindow(top, { [SCRAMJETFRAME]: sframe }, true);
	const leaf = makeWindow(mid, {}, true);
	t.is(frameOf(leaf), sframe);
});

test("ancestor without a scramjet client does not throw (nested host shell)", (t) => {
	// shell (no client) -> tabs page (no client) -> proxied page (client, plain iframe)
	const shell = makeWindow(null, null, false);
	const tabs = makeWindow(shell, {}, false);
	const proxied = makeWindow(tabs, {}, true);
	t.notThrows(() => frameOf(proxied));
	t.is(frameOf(proxied), null);
});
