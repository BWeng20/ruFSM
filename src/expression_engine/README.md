# Expression Datamodel

This sub project implements a fast and simple W3C-SCXML Datamodel.
It an expression-like, non-Turing-complete language. 

It is available if feature _"RfsmExpressionModel"_ is turned on.

### Selection of Datamodel

To select this model in SCXML use `datamodel="rfsm-expression"`. 

### Custom Actions

Custom action via the trait "Action" can be called like methods.

_Like global functions_

```
  length("a string")
```

_Like member-functions_<br/>
In this case the value on which this action is called is given as first argument.
This works for all actions with at least one argument.

```
  "a string".length()
```

There are several pre-defined Actions:

| Action     | Arguments                                                                                                       | Return value  | Description                                                                                    |
|------------|-----------------------------------------------------------------------------------------------------------------|---------------|------------------------------------------------------------------------------------------------|
| length     | One argument of type <ul><li>Data::String</li><li>Data::Array</li><li>Data::Map</li><li>Data::Source</li></ul> | Data::Integer |                                                                                                |
| isDefined  | One argument of any kind.                                                                                       | Data::Boolean | Checks if the argument not Data::Error or Data::None.                                          |
| indexOf    | One argument of type <ul><li>Data::String</li><li>Data::Array</li><li>Data::Map</li><li>Data::Source</li></ul>                                                                                                                |               |                                                                                                |
| In         | One argument of type Data::String.                                                                              | Data::Boolean | Implements SCXML "In" function. Checks if the given state is inside the current configuration. |

