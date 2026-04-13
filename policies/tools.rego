package tools

import rego.v1

default allow := false

allow if {
    input.tool != "shell"
    input.tool != "shell_executor"
}

allow if {
    input.tool == "shell"
    input.user.role == "admin"
}

allow if {
    input.tool == "shell_executor"
    input.user.role == "admin"
}
