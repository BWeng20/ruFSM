# Expression Datamodel

This module implements a fast and simple W3C-SCXML Datamodel.
It an expression-like, non-Turing-complete language. 

It is available if feature _"RfsmExpressionModel"_ is turned on.

### Selection of Datamodel

To select this model in SCXML use `datamodel="rfsm-expression"`. 

### Syntax

```
  <expression>     ::= <sub-expression> [<operator> <expression>]
  <sub-expression> ::= {"!"}<field>{.<field>}
  <field>         ::= <identifier>["(" <arguments> ")"]
  <arguments>      ::= <sub-expression>{"," <sub-expression>}
  <identifier>     ::= <letter>{<letter>|<digit>|"_"}
  <operator>       ::= "?=" | "=" | "==" | ">=" | "<=" | "*" | "%" | "+" | "-" | ":" | "/" | "&" | "|"
  <digit>          ::= "0" .. "9"  
  <letter>         ::= "A" .. "Z" | "a" .. "z"  
  <number>         ::= <integer> <fraction> <exponent>
  <integer>        ::= ["-"]( ("1".."9"{<digit>}) | "0" ) 
  <fraction>       ::= "" | "." <digit>{<digit>}
  <exponent>       := "" | ("E"|"e")[+|-]<digit>{<digit>}
```

Numbers are represented as specified in JSON.

### Operators

The available operators and their meaning

| Operator             | Name           | Description                                                                                                          |
|----------------------|----------------|----------------------------------------------------------------------------------------------------------------------|
| `=`                  | Assignment     | The result of the right side is assigned to the left side. Left side must specify an existing writable variable.     |
| `?=`                 | Initialisation | The left side is created and initialized with the result of the right side. Left side specifies a writable variable. |                                                 |
| `==`                 | Equals         | Results to `true` if the left side equals the right side.                                                            |
| `>=`, `<=`, `>`, `<` | Comparison     | Results to `true` if left and right satisfies the condition.                                                         |
| `/`, `:`             | Division       | Works only on numeric types. Returns a Data::Double if at least one operant is Double, otherwise Data::Integer.      |
| `+`                  | Aggregation    | Computes the sum for Data::Integer or Data::Double and the aggregation for Data::Map and Data::Array.                |
| `-`                  | Minus          | Computes the difference of left and right. Works only on numeric types.                                              |
| `%`                  | Modulus        | Computes the remainder of the of dividing left by right. Works only on numeric types.                                |


### Custom Actions

Custom actions via the trait "Action" can be called like methods.

_Call them like global functions_

```
  length("a string")
```

_Call them like member-functions_<br/>
In this case, the value on which this action is called is given as first argument.
This works for all actions with at least one argument.

```
  "a string".length()
```

There are several pre-defined Actions:

| Action    | Arguments                                                                                                      | Return value  | Description                                                                                    |
|-----------|----------------------------------------------------------------------------------------------------------------|---------------|------------------------------------------------------------------------------------------------|
| abs       | One argument of type <ul><li>Data::Double</li><li>Data::Integer</li></ul>                                      | Same as input | Computes the absolute value.                                                                   |
| length    | One argument of type <ul><li>Data::String</li><li>Data::Array</li><li>Data::Map</li><li>Data::Source</li></ul> | Data::Integer | Get the length of the argument.                                                                |
| isDefined | One argument of any kind.                                                                                      | Data::Boolean | Technical this checks if the argument not Data::Error or Data::None.                           |
| indexOf   | Two arguments of type Data::String.                                                                            | Data::Integer | Get the index of the second string inside the first one. Returns -1, if the string not found.  |
| In        | One argument of type Data::String.                                                                             | Data::Boolean | Implements SCXML "In" function. Checks if the given state is inside the current configuration. |
