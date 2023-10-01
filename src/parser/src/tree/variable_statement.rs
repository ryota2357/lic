use super::*;

#[derive(Clone, Debug, PartialEq)]
pub enum VariableStatement<'src> {
    Var {
        lhs: Local<'src>,
        rhs: Expression<'src>,
    },
    Let {
        lhs: Local<'src>,
        rhs: Expression<'src>,
    },
    Func {
        name: Local<'src>,
        args: Vec<Ident<'src>>,
        body: Chunk<'src>,
    },
    Assign {
        lhs: Local<'src>,
        rhs: Expression<'src>,
    },
}

/// <VariableStatement> ::= <Var> | <Let> | <Func> | <Assign>
/// <Var>               ::= 'var' <Local> '=' <Expression>
/// <Let>               ::= 'let' <Local> '=' <Expression>
/// <Func>              ::= 'func' <Local> '(' [ <Ident> { ',' <Ident> } [ ',' ] ] ')' <Block> 'end'
/// <Assign>            ::= <Local> '=' <Expression>
pub(super) fn variable_statement<'tokens, 'src: 'tokens>(
    block: impl Parser<'tokens, ParserInput<'tokens, 'src>, Block<'src>, ParserError<'tokens, 'src>>
        + Clone
        + 'tokens,
    expression: impl Parser<'tokens, ParserInput<'tokens, 'src>, Expression<'src>, ParserError<'tokens, 'src>>
        + Clone
        + 'tokens,
) -> impl Parser<
    'tokens,
    ParserInput<'tokens, 'src>,
    VariableStatement<'src>,
    ParserError<'tokens, 'src>,
> + Clone {
    let var = just(Token::Var)
        .ignore_then(local())
        .then_ignore(just(Token::Assign))
        .then(expression.clone())
        .map(|(lhs, rhs)| VariableStatement::Var { lhs, rhs });
    let r#let = just(Token::Let)
        .ignore_then(local())
        .then_ignore(just(Token::Assign))
        .then(expression.clone())
        .map(|(lhs, rhs)| VariableStatement::Let { lhs, rhs });
    let func = just(Token::Func)
        .ignore_then(local())
        .then_ignore(just(Token::OpenParen))
        .then(
            ident()
                .separated_by(just(Token::Comma))
                .allow_trailing()
                .collect(),
        )
        .then_ignore(just(Token::CloseParen))
        .then(block)
        .then_ignore(just(Token::End))
        .map(|((name, args), block)| VariableStatement::Func {
            name,
            args,
            body: block.into(),
        });
    let assign = local()
        .then_ignore(just(Token::Assign))
        .then(expression)
        .map(|(lhs, rhs)| VariableStatement::Assign { lhs, rhs });

    var.or(r#let).or(func).or(assign)
}

impl<'a> TreeWalker<'a> for VariableStatement<'a> {
    fn analyze(&mut self, tracker: &mut Tracker<'a>) {
        match self {
            VariableStatement::Var { lhs, rhs } => {
                match lhs {
                    Local::TableField { name, .. } => tracker.add_capture(name.str),
                    Local::Variable { name } => tracker.add_definition(name.str),
                }
                rhs.analyze(tracker);
            }
            VariableStatement::Let { lhs, rhs } => {
                match lhs {
                    Local::TableField { name, .. } => tracker.add_capture(name.str),
                    Local::Variable { name } => tracker.add_definition(name.str),
                }
                rhs.analyze(tracker);
            }
            VariableStatement::Func { name, args, body } => {
                match name {
                    Local::TableField { name, .. } => tracker.add_capture(name.str),
                    Local::Variable { name } => tracker.add_definition(name.str),
                }

                tracker.push_new_definition_scope();
                for arg in args.iter() {
                    tracker.add_definition(arg.str);
                }
                body.analyze(tracker);
                tracker.pop_current_definition_scope();
            }
            VariableStatement::Assign { lhs, rhs } => {
                match lhs {
                    Local::TableField { name, .. } => tracker.add_capture(name.str),
                    Local::Variable { name } => tracker.add_capture(name.str),
                }
                rhs.analyze(tracker);
            }
        }
    }
}
