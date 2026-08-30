# fers

A small stack-based (concatenative) language, written in Rust. Not
meant to be practically useful — it's a design exercise — but it
works end to end: parsing, evaluation, closures, macros.

## Basics

Values are pushed onto a stack and consumed by operators/functions in
postfix (RPN) order:

```
2 3 +        # pushes 2, 3, then adds -> 5
```

## Line-scoped stacks

Unlike most concatenative languages (Forth, Factor...), each **line**
has its own isolated stack — the stack is cleared at every line break.
The `$` operator copies the *entire* stack from the previous line,
letting you carry results forward explicitly instead of sharing one
continuous stack:

```
println: string "\n" + write

let $fun = ($x) -> {
    2 $x *
    $ $
}

2 ($fun) + println
# => 8   (line 1: [4], line 2: $ $ -> [4, 4], summed outside the block)
```

## Types

`integer`(`i128`), `float` (backed by `rust_decimal`, so exact
decimal arithmetic rather than IEEE 754 binary floats), `string`,
`bool`, `null`, and `closure` (closures are first-class values). 
Fers is strongly typed and does not perform implicit conversions 
in general — except for two cases:

- Numeric promotion: mixing `integer` and `float` in an operation
  promotes to `float`.
- String coercion: if one operand is a `string`, the other is
  converted to its string representation.

```
"a" 1 +       # "a1"   (string coercion)
1 2.5 +       # 3.5    (int/float promotion)
```

`null` propagates through operations instead of raising an error:

```
null 1 +      # null
10 0 /        # null, division by zero
"a" 30 *      # "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"  (string repeat)
```

## Variables can hold multiple values

A `let` binding captures the *entire* current stack at the point of
declaration — not just the top value. Using the variable pushes all
of its values back at once:

```
let $x = 2 3
$x +          # pushes 2, 3 -> 5
```

## Closures vs macros

- **Closures** (`let $f = (args) -> { ... }`) capture their *defining*
  scope (lexical, parent-site) plus their declared arguments.
- **Macros** (`name: ...`) are just a sequence of instructions with
  no context of their own — they're expanded inline at their call
  site. A macro's body doesn't have to produce a closure (e.g.
  `println` is a plain macro); when it does, any captures inside that
  closure resolve at the macro's *expansion* site, not where the
  macro itself was defined.

```
+F: (($f $g) -> {
    ($x) -> {
        $x ($f) $x ($g) +
    }
})

let $f = ($x) -> { $x 1 + }
let $g = ($x) -> { $x 2 * }

let $h = $f $g +F
10 ($h) println
```

## Memory model

No garbage collector. Values are reference-counted and freed via RAII
when their owning scope ends (end of stack, end of closure).

## Implementation

Two-pass front end: a lexer, then a parser producing an AST. The
interpreter is a straightforward AST walker — no bytecode compilation,
no VM.

## Roadmap

- **String templating** — Swift-style interpolation is being worked
  on (`` `Hello \($name)` ``), but not implemented yet; the parser errors
  currently on this syntax.

## Building

```bash
cargo build --release
./target/release/fers run script.fers
```
