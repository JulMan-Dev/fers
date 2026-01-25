pub(crate) mod expression;
pub(crate) mod token;
pub(crate) mod statement;
pub(crate) mod macro_def;
pub mod ast;
pub(crate) mod meta;
pub(crate) mod variable;
pub(crate) mod utils;

use crate::{
    parser::{ast::Chunk, statement::consume_statement},
    error::{ErrorKind, ErrorStack},
    token::TokenList
};

static KEYWORDS: [&str; 18] = [
    "clone", "type", "+", "-", "*", "/", "neg", "integer", "float", "string", "boolean", "eval",
    "parse", "error", "panic", "let", "=", ":"
];

pub fn parse(input: &TokenList) -> Result<Chunk, ErrorStack> {
    let mut i = 0;
    let mut statements = Vec::new();
    
    loop {
        let result = consume_statement(input, i);
        
        match result {
            Ok((consumed, statement)) => {
                statements.push(statement);
                i += consumed;
            },
            Err(stack) if matches!(stack.kind(), ErrorKind::EndOfFile) => {
                break Ok(Chunk {
                    statements,
                    source: input.0.clone(),
                });
            },
            Err(stack) => break Err(stack),
        }
    }
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
