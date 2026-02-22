# Limitations

## Grammar Definition

### Binding Anonymous Groups
Directly binding to an anonymous group with an action block is not supported.
`rule main = x:("a" -> { 1 })` will fail to compile.
Instead, extract the group into a named rule:
`rule main = x:my_rule`
`rule my_rule = "a" -> { 1 }`
