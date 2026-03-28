pub(crate) mod expression;
pub(crate) mod token;
pub(crate) mod statement;
pub(crate) mod macro_def;
pub mod ast;
pub(crate) mod meta;
pub(crate) mod variable;
pub(crate) mod utils;
pub(crate) mod closure;

use crate::{
    parser::{ast::Chunk, statement::consume_statement},
    error::{ErrorKind, ErrorStack},
    token::TokenList
};
use crate::token::RealToken;

static KEYWORDS: [&str; 18] = [
    "clone", "type", "+", "-", "*", "/", "neg", "integer", "float", "string", "boolean", "eval",
    "parse", "error", "panic", "let", "=", ":"
];

pub fn parse(input: &TokenList) -> Result<Chunk, ErrorStack> {
    match consume_chunk(input, 0) {
        Ok((consumed, chunk)) => {
            println!("{:#?}", chunk);
            
            // check if remaining tokens after EOF
            if input.1.len() > consumed {
                return Err(ErrorStack::new(
                    ErrorKind::UnexpectedToken,
                    input.0.clone(),
                    (input.1[consumed].2..).into(),
                ));
            }
            
            Ok(chunk)
        },
        Err(stack) => Err(stack),
    }
}

/// This function should not be used to parse an entire file. Consume statements
/// until EOF or "}" at the first of a line.
/// 
/// Notes:
///  - this function NEVER returns [`ErrorKind::EndOfFile`].
///  - this function DOES NOT consume the final "}".
/// 
/// The return type is not [`Option`] because a chunk can be empty.
/// 
/// Returns a chunk and the number of consumed tokens on success, or an error stack
/// if it fails.
pub fn consume_chunk(input: &TokenList, mut i: usize) -> Result<(usize, Chunk), ErrorStack> {
    let mut statements = Vec::new();
    let mut consumed = 0;

    let chunk = loop {
        // if "}", we are ending a chunk, return it.
        if let Some(token) = input.1.get(i) && 
            let RealToken::Unknown(s) = &token.0 && 
            s.as_ref() == "}" {
            break Ok(Chunk {
                statements,
                source: input.0.clone(),
            });
        }
        
        let result = consume_statement(input, i);

        match result {
            Ok((k, statement)) => {
                statements.push(statement);
                consumed += k;
                i += k;
            },
            Err(stack) if matches!(stack.kind(), ErrorKind::EndOfFile | ErrorKind::EndOfBlock) => {
                break Ok(Chunk {
                    statements,
                    source: input.0.clone(),
                });
            },
            Err(stack) => break Err(stack),
        }
    }?;
    
    Ok((consumed, chunk))
}

// #[derive(Debug, PartialEq, Eq, Clone)]
// enum PathAction {
//     PopVisit,
//     NewVisit(Rc<str>),
// }
// 
// fn check_for_macro_recursion(
//     macro_name: &Rc<str>,
//     macros: &HashMap<Rc<str>, HashSet<Rc<str>>>,
//     current_macro: &HashSet<Rc<str>>,
// ) -> Option<ErrorKind> {
//     use PathAction::*;
// 
//     let mut path = VecDeque::from([macro_name.clone()]);
//     let mut visited = HashSet::from([macro_name.clone()]);
//     let mut queue: VecDeque<PathAction> = current_macro.iter().cloned().map(NewVisit).collect();
// 
//     while let Some(action) = queue.pop_back() {
//         match action {
//             // pop last path node
//             PopVisit => {
//                 path.pop_back();
//             }
// 
//             // visiting new macro
//             NewVisit(name) => {
//                 // node already in path, cycle, returning error
//                 if path.contains(&name) {
//                     path.push_back(macro_name.clone());
//                     return Some(ErrorKind::RecursiveMacroInclusion(
//                         path.iter().cloned().collect(),
//                     ));
//                 }
// 
//                 path.push_back(name.clone());
//                 // we enforce checking new visited macro
//                 queue.push_back(PopVisit);
// 
//                 // already visited, node, no cycle possible, continue
//                 if visited.contains(&name) {
//                     continue;
//                 }
// 
//                 visited.insert(name.clone());
// 
//                 if let Some(deps) = macros.get(&name) {
//                     queue.extend(deps.iter().cloned().map(NewVisit));
//                     // queue.push_back(NewVisit(name));
//                 }
//             }
//         }
//     }
// 
//     None
// }
