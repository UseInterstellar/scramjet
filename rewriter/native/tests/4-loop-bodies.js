// loop bodies must be rewritten like any other statement (regression: handle_for_of_in never walked them)
function g() { return [1]; }
const obj = { a: 1 };

for (const x of g()) {
  check(top);
  check(location);
  check(window.location);
}

for (let k in obj) {
  check(top);
  check(location);
}

// nested, and as a bare (brace-less) body
for (const x of g()) for (const y of g()) check(location);
for (const x of g()) for (const k in obj) check(top["loca" + "tion"]);

// destructuring heads still work
for (const [a, b] of [[1, 2]]) {
  assert(a === 1 && b === 2);
  check(top);
}
for (const { a } of [obj]) {
  assert(a === 1);
  check(location);
}

// plain old loops are unaffected
for (let i = 0; i < 1; i++) check(top);

(async () => {
  for await (const x of g()) {
    check(location);
    check(top);
  }
})();
