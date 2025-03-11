// use crate::ast_nodes::*;
// use crate::lexer::Token;

// pub struct Parser<'a> {
//     tokens: &'a [Token<'a>],
//     current: usize,
// }

// impl<'a> Parser<'a> {
//     pub fn new(tokens: &'a [Token]) -> Self {
//         Self { tokens, current: 0 }
//     }

//     pub fn parse(&mut self) -> Result<Vec<Command<'a>>, String> {
//         let mut commands = Vec::new();

//         while !self.is_at_end() {
//             let command = self.parse_command()?;
//             commands.push(command);
//         }

//         Ok(commands)
//     }

//     fn parse_command(&mut self) -> Result<Command<'a>, String> {
//         let token = self.advance();

//         match &token.token_type() {
//             TokenType::Read => {
//                 let ident_token = self.advance();
//                 match &ident_token.token_type() {
//                     TokenType::Read => Ok(Command::Read {
//                         position: token.position(),
//                         destination: LValue::Variable(ident_token),
//                         source: Expression::Variable(ident_token),
//                     }),
//                     _ => Err("Expected identifier after 'read'".to_string()),
//                 }
//             }
//             _ => Err("Unexpected token while parsing command".to_string()),
//         }
//     }

//     fn is_at_end(&self) -> bool {
//         self.current >= self.tokens.len()
//     }

//     fn advance(&mut self) -> &Token {
//         let token = &self.tokens[self.current];
//         self.current += 1;
//         token
//     }
// }

// pub trait ASTVisitor<'a> {
//     fn visit_command(&mut self, command: &Command<'a>);
//     fn visit_lvalue(&mut self, lval: &LValue<'a>);
//     fn visit_statement(&mut self, stmt: &Statement<'a>);
//     fn visit_expression(&mut self, expr: &Expression<'a>);
// }

// pub struct RecursiveVisitor;

// impl<'a> ASTVisitor<'a> for RecursiveVisitor {
//     fn visit_command(&mut self, command: &Command<'a>) {
//         match command {
//             Command::Read { destination, .. } => {
//                 self.visit_lvalue(destination);
//             }
//         }
//     }

//     fn visit_lvalue(&mut self, lval: &LValue<'a>) {
//         match lval {
//             LValue::Variable(name) => {
//                 println!("Visiting variable: {}", name);
//             }
//         }
//     }

//     fn visit_statement(&mut self, stmt: &Statement<'a>) {
//         match stmt {
//             Statement::Command(cmd) => self.visit_command(cmd),
//             Statement::Empty => {}
//         }
//     }

//     fn visit_expression(&mut self, expr: &Expression<'a>) {
//         match expr {
//             Expression::Variable(name) => {
//                 println!("Visiting expr var: {}", name);
//             }
//         }
//     }
// }

// impl<'a> Command<'a> {
//     pub fn accept<V: ASTVisitor<'a>>(&self, visitor: &mut V) {
//         visitor.visit_command(self);
//     }
// }
