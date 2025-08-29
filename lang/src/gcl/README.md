# Guarded Command Language

## Language syntax
```ebnf
Program       := Statement*
Statement     := Skip                               // No operation 
               | <name> := Expr                     // Variable assignment      
               | <name> [ Expr ] := Expr            // Array element assignment
               | if Expr then Statement [ else Statement ] fi  // Conditional statement
               | while Expr do Statement od         // Loop statement
               | Statement ; Statement              // Sequential composition
Expr          := Literal                            // Literal values
               | <name>                             // Variable reference
               | Expr Op Expr                       // Binary operators
               | not Expr                           // Unary negation (boolean NOT)
Literal       := <int> | true | false
Op            := + | - | * | /                      // Arithmetic ops (Int)
               | > | >= | < | <= | == | !=          // Comparison ops (Int or Bool equality)
               | and | or | xor                     // Boolean ops (AND, OR, XOR)
Type          := Int | Bool                         // Variable types
```

## Language semantics

SSA (Static Single Assignment) form is used for variable assignments. In other words, each variable is assigned at most once in any path through the program. We now outline the operational semantics (at a high level) for each construct and the runtime behavior. The program state consists of a mapping from variables to values. Initially, all declared variables are undefined (no value). We model an undefined value as a special marker (denoted Undef). During execution, variables will get assigned exactly once (on each path) as per SSA discipline. If an undefined variable is ever used in an expression before assignment, that is a runtime error.

### Type system

- `Bool`:
    - Literal: `true`, `false`
    - `! Bool`
    - `Bool & Bool`, `Bool | Bool`, `Bool ^ Bool`
    - `Bool == Bool`, `Bool != Bool`
    - `Int > Int`, `Int >= Int`, `Int <= Int`, `Int < Int`, `Int == Int`, `Int != Int`

- `Int`:
    - Literal: `<nat>`
    - `Int + Int`, `Int - Int`, `Int * Int`
    - `Int / Int` yields either `Int` or *divide-by-zero* runtime error

### Example program

```while
x := 5;
y := 10;
if x < y then
    x := x + 1;
else
    y := y - 1;
fi;
while x < y do
    x := x + 2;
od;
```

