package tools

import rego.v1

default allow := false

allow if {
    input.tool != "shell"
}

allow if {
    input.tool == "shell"
    input.user.role == "admin"
}
