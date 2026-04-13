package tools

default allow = false

allow {
    input.tool != "shell"
}

allow {
    input.tool == "shell"
    input.user.role == "admin"
}
