## all output im backtivks

UI needs output in 4 backticks e.g.

```` I will run `cargo test` ````

## experimente

ask user feedback before dooing experiments, sometimes user can give information 

## file writes

use default_api::write_file to write files

## grep 

never grep in root because binary files in subdirectories.

## Error Output

User always want to see the error output.

## Collaboration with the User

*   **User is the source of truth:** The user's explicit instructions, commands, and feedback are the ultimate source of truth, superseding any interpretation of the project code.
*   **Follow the user's lead:** My role is to be a tool that follows the user's direction. The user leads; I assist. I must not take unsolicited major actions like deleting projects or replacing implementations.
*   **Format for Utility:** All error messages, logs, and code snippets must be enclosed in `backticks` to make them easy for the user to copy and paste.
