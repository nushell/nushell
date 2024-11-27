use nu_engine::command_prelude::*;
use nu_protocol::ast::{Assignment, Bits, Boolean, Comparison, Math, Operator};

#[derive(Clone)]
pub struct HelpOperators;

impl Command for HelpOperators {
    fn name(&self) -> &str {
        "help operators"
    }

    fn description(&self) -> &str {
        "Show help on nushell operators."
    }

    fn signature(&self) -> Signature {
        Signature::build("help operators")
            .category(Category::Core)
            .input_output_types(vec![(Type::Nothing, Type::table())])
            .allow_variants_without_examples(true)
    }

    fn run(
        &self,
        _engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let head = call.head;
        let mut operators = [
            Operator::Assignment(Assignment::Assign),
            Operator::Assignment(Assignment::PlusAssign),
            Operator::Assignment(Assignment::ConcatAssign),
            Operator::Assignment(Assignment::MinusAssign),
            Operator::Assignment(Assignment::MultiplyAssign),
            Operator::Assignment(Assignment::DivideAssign),
            Operator::Comparison(Comparison::Equal),
            Operator::Comparison(Comparison::NotEqual),
            Operator::Comparison(Comparison::LessThan),
            Operator::Comparison(Comparison::GreaterThan),
            Operator::Comparison(Comparison::LessThanOrEqual),
            Operator::Comparison(Comparison::GreaterThanOrEqual),
            Operator::Comparison(Comparison::RegexMatch),
            Operator::Comparison(Comparison::NotRegexMatch),
            Operator::Comparison(Comparison::In),
            Operator::Comparison(Comparison::NotIn),
            Operator::Comparison(Comparison::StartsWith),
            Operator::Comparison(Comparison::EndsWith),
            Operator::Math(Math::Plus),
            Operator::Math(Math::Concat),
            Operator::Math(Math::Minus),
            Operator::Math(Math::Multiply),
            Operator::Math(Math::Divide),
            Operator::Math(Math::Modulo),
            Operator::Math(Math::FloorDivision),
            Operator::Math(Math::Pow),
            Operator::Bits(Bits::BitOr),
            Operator::Bits(Bits::BitXor),
            Operator::Bits(Bits::BitAnd),
            Operator::Bits(Bits::ShiftLeft),
            Operator::Bits(Bits::ShiftRight),
            Operator::Boolean(Boolean::And),
            Operator::Boolean(Boolean::Or),
            Operator::Boolean(Boolean::Xor),
        ]
        .into_iter()
        .map(|op| {
            Value::record(
                record! {
                    "type" => Value::string(op_type(&op), head),
                    "operator" => Value::string(op.to_string(), head),
                    "name" => Value::string(name(&op), head),
                    "description" => Value::string(description(&op), head),
                    "precedence" => Value::int(op.precedence().into(), head),
                },
                head,
            )
        })
        .collect::<Vec<_>>();

        operators.push(Value::record(
            record! {
                "type" => Value::string("Boolean", head),
                "operator" => Value::string("not", head),
                "name" => Value::string("Not", head),
                "description" => Value::string("Negates a value or expression.", head),
                "precedence" => Value::int(0, head),
            },
            head,
        ));

        Ok(Value::list(operators, head).into_pipeline_data())
    }
}

fn op_type(operator: &Operator) -> &'static str {
    match operator {
        Operator::Comparison(_) => "Comparison",
        Operator::Math(_) => "Math",
        Operator::Boolean(_) => "Boolean",
        Operator::Bits(_) => "Bitwise",
        Operator::Assignment(_) => "Assignment",
    }
}

fn name(operator: &Operator) -> String {
    match operator {
        Operator::Comparison(op) => format!("{op:?}"),
        Operator::Math(op) => format!("{op:?}"),
        Operator::Boolean(op) => format!("{op:?}"),
        Operator::Bits(op) => format!("{op:?}"),
        Operator::Assignment(op) => format!("{op:?}"),
    }
}

fn description(operator: &Operator) -> &'static str {
    // NOTE: if you have to add an operator here, also add it to the operator list above.
    match operator {
        Operator::Comparison(Comparison::Equal) => "Checks if two values are equal.",
        Operator::Comparison(Comparison::NotEqual) => "Checks if two values are not equal.",
        Operator::Comparison(Comparison::LessThan) => "Checks if a value is less than another.",
        Operator::Comparison(Comparison::GreaterThan) => {
            "Checks if a value is greater than another."
        }
        Operator::Comparison(Comparison::LessThanOrEqual) => {
            "Checks if a value is less than or equal to another."
        }
        Operator::Comparison(Comparison::GreaterThanOrEqual) => {
            "Checks if a value is greater than or equal to another."
        }
        Operator::Comparison(Comparison::RegexMatch) => {
            "Checks if a value matches a regular expression."
        }
        Operator::Comparison(Comparison::NotRegexMatch) => {
            "Checks if a value does not match a regular expression."
        }
        Operator::Comparison(Comparison::In) => {
            "Checks if a value is in a list, is part of a string, or is a key in a record."
        }
        Operator::Comparison(Comparison::NotIn) => {
            "Checks if a value is not in a list, is not part of a string, or is not a key in a record."
        }
        Operator::Comparison(Comparison::StartsWith) => "Checks if a string starts with another.",
        Operator::Comparison(Comparison::EndsWith) => "Checks if a string ends with another.",
        Operator::Math(Math::Plus) => "Adds two values.",
        Operator::Math(Math::Concat) => {
            "Concatenates two lists, two strings, or two binary values."
        }
        Operator::Math(Math::Minus) => "Subtracts two values.",
        Operator::Math(Math::Multiply) => "Multiplies two values.",
        Operator::Math(Math::Divide) => "Divides two values.",
        Operator::Math(Math::Modulo) => "Divides two values and returns the remainder.",
        Operator::Math(Math::FloorDivision) => "Divides two values and floors the result.",
        Operator::Math(Math::Pow) => "Raises one value to the power of another.",
        Operator::Boolean(Boolean::And) => "Checks if both values are true.",
        Operator::Boolean(Boolean::Or) => "Checks if either value is true.",
        Operator::Boolean(Boolean::Xor) => "Checks if one value is true and the other is false.",
        Operator::Bits(Bits::BitOr) => "Performs a bitwise OR on two values.",
        Operator::Bits(Bits::BitXor) => "Performs a bitwise XOR on two values.",
        Operator::Bits(Bits::BitAnd) => "Performs a bitwise AND on two values.",
        Operator::Bits(Bits::ShiftLeft) => "Bitwise shifts a value left by another.",
        Operator::Bits(Bits::ShiftRight) => "Bitwise shifts a value right by another.",
        Operator::Assignment(Assignment::Assign) => "Assigns a value to a variable.",
        Operator::Assignment(Assignment::PlusAssign) => "Adds a value to a variable.",
        Operator::Assignment(Assignment::ConcatAssign) => {
            "Concatenates two lists, two strings, or two binary values."
        }
        Operator::Assignment(Assignment::MinusAssign) => "Subtracts a value from a variable.",
        Operator::Assignment(Assignment::MultiplyAssign) => "Multiplies a variable by a value.",
        Operator::Assignment(Assignment::DivideAssign) => "Divides a variable by a value.",
    }
}

#[cfg(test)]
mod test {
    #[test]
    fn test_examples() {
        use super::HelpOperators;
        use crate::test_examples;
        test_examples(HelpOperators {})
    }
}
