## about winnow 0.7

Replace ModalResult<O, E> with winnow::Result<O> (if using default error type) or just Result<O, E>
Replace impl ModalParser with impl Parser
Remove uses of ErrMode.

### Additional Info

- Decoupled ErrMode from the core traits through new ModalError trait and ParserError modal functions, allowing better performance and greater flexibility
- Add ParserError, AsChar, ContainsToken, Stream to the prelude
- Add stream::TokenSlice to help parsing of lexed tokens
- Implement ErrorConvert for ErrMode

## experimente

ask user feedback before dooing experiments, sometimes user can give information 

## file writes

use default_api::write_file to write files

## grep 

never grep in root because binary files in subdirectories.

