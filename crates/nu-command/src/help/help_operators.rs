use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct HelpOperators;

impl Command for HelpOperators {
    fn name(&self) -> &str {
        "help operators"
    }

    fn usage(&self) -> &str {
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
        engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let head = call.head;
        let op_info = generate_operator_info();
        let mut recs = vec![];

        for op in op_info {
            recs.push(Value::record(
                record! {
                    "type" => Value::string(op.op_type, head),
                    "operator" => Value::string(op.operator, head),
                    "name" => Value::string(op.name, head),
                    "description" => Value::string(op.description, head),
                    "precedence" => Value::int(op.precedence, head),
                },
                head,
            ));
        }

        Ok(recs
            .into_iter()
            .into_pipeline_data(engine_state.ctrlc.clone()))
    }
}

struct OperatorInfo {
    op_type: String,
    operator: String,
    name: String,
    description: String,
    precedence: i64,
}

fn generate_operator_info() -> Vec<OperatorInfo> {
    vec![
        OperatorInfo {
            op_type: "Assignment".into(),
            operator: "=".into(),
            name: "Assign".into(),
            description: "Assigns a value to a variable.".into(),
            precedence: 10,
        },
        OperatorInfo {
            op_type: "Assignment".into(),
            operator: "+=".into(),
            name: "PlusAssign".into(),
            description: "Adds a value to a variable.".into(),
            precedence: 10,
        },
        OperatorInfo {
            op_type: "Assignment".into(),
            operator: "++=".into(),
            name: "AppendAssign".into(),
            description: "Appends a list or a value to a variable.".into(),
            precedence: 10,
        },
        OperatorInfo {
            op_type: "Assignment".into(),
            operator: "-=".into(),
            name: "MinusAssign".into(),
            description: "Subtracts a value from a variable.".into(),
            precedence: 10,
        },
        OperatorInfo {
            op_type: "Assignment".into(),
            operator: "*=".into(),
            name: "MultiplyAssign".into(),
            description: "Multiplies a variable by a value.".into(),
            precedence: 10,
        },
        OperatorInfo {
            op_type: "Assignment".into(),
            operator: "/=".into(),
            name: "DivideAssign".into(),
            description: "Divides a variable by a value.".into(),
            precedence: 10,
        },
        OperatorInfo {
            op_type: "Comparison".into(),
            operator: "==".into(),
            name: "Equal".into(),
            description: "Checks if two values are equal.".into(),
            precedence: 80,
        },
        OperatorInfo {
            op_type: "Comparison".into(),
            operator: "!=".into(),
            name: "NotEqual".into(),
            description: "Checks if two values are not equal.".into(),
            precedence: 80,
        },
        OperatorInfo {
            op_type: "Comparison".into(),
            operator: "<".into(),
            name: "LessThan".into(),
            description: "Checks if a value is less than another.".into(),
            precedence: 80,
        },
        OperatorInfo {
            op_type: "Comparison".into(),
            operator: "<=".into(),
            name: "LessThanOrEqual".into(),
            description: "Checks if a value is less than or equal to another.".into(),
            precedence: 80,
        },
        OperatorInfo {
            op_type: "Comparison".into(),
            operator: ">".into(),
            name: "GreaterThan".into(),
            description: "Checks if a value is greater than another.".into(),
            precedence: 80,
        },
        OperatorInfo {
            op_type: "Comparison".into(),
            operator: ">=".into(),
            name: "GreaterThanOrEqual".into(),
            description: "Checks if a value is greater than or equal to another.".into(),
            precedence: 80,
        },
        OperatorInfo {
            op_type: "Comparison".into(),
            operator: "=~".into(),
            name: "RegexMatch".into(),
            description: "Checks if a value matches a regular expression.".into(),
            precedence: 80,
        },
        OperatorInfo {
            op_type: "Comparison".into(),
            operator: "!~".into(),
            name: "NotRegexMatch".into(),
            description: "Checks if a value does not match a regular expression.".into(),
            precedence: 80,
        },
        OperatorInfo {
            op_type: "Comparison".into(),
            operator: "in".into(),
            name: "In".into(),
            description: "Checks if a value is in a list or string.".into(),
            precedence: 80,
        },
        OperatorInfo {
            op_type: "Comparison".into(),
            operator: "not-in".into(),
            name: "NotIn".into(),
            description: "Checks if a value is not in a list or string.".into(),
            precedence: 80,
        },
        OperatorInfo {
            op_type: "Comparison".into(),
            operator: "starts-with".into(),
            name: "StartsWith".into(),
            description: "Checks if a string starts with another.".into(),
            precedence: 80,
        },
        OperatorInfo {
            op_type: "Comparison".into(),
            operator: "ends-with".into(),
            name: "EndsWith".into(),
            description: "Checks if a string ends with another.".into(),
            precedence: 80,
        },
        OperatorInfo {
            op_type: "Comparison".into(),
            operator: "not".into(),
            name: "UnaryNot".into(),
            description: "Negates a value or expression.".into(),
            precedence: 0,
        },
        OperatorInfo {
            op_type: "Math".into(),
            operator: "+".into(),
            name: "Plus".into(),
            description: "Adds two values.".into(),
            precedence: 90,
        },
        OperatorInfo {
            op_type: "Math".into(),
            operator: "++".into(),
            name: "Append".into(),
            description: "Appends two lists or a list and a value.".into(),
            precedence: 80,
        },
        OperatorInfo {
            op_type: "Math".into(),
            operator: "-".into(),
            name: "Minus".into(),
            description: "Subtracts two values.".into(),
            precedence: 90,
        },
        OperatorInfo {
            op_type: "Math".into(),
            operator: "*".into(),
            name: "Multiply".into(),
            description: "Multiplies two values.".into(),
            precedence: 95,
        },
        OperatorInfo {
            op_type: "Math".into(),
            operator: "/".into(),
            name: "Divide".into(),
            description: "Divides two values.".into(),
            precedence: 95,
        },
        OperatorInfo {
            op_type: "Math".into(),
            operator: "//".into(),
            name: "FloorDivision".into(),
            description: "Divides two values and floors the result.".into(),
            precedence: 95,
        },
        OperatorInfo {
            op_type: "Math".into(),
            operator: "mod".into(),
            name: "Modulo".into(),
            description: "Divides two values and returns the remainder.".into(),
            precedence: 95,
        },
        OperatorInfo {
            op_type: "Math".into(),
            operator: "**".into(),
            name: "Pow ".into(),
            description: "Raises one value to the power of another.".into(),
            precedence: 100,
        },
        OperatorInfo {
            op_type: "Bitwise".into(),
            operator: "bit-or".into(),
            name: "BitOr".into(),
            description: "Performs a bitwise OR on two values.".into(),
            precedence: 60,
        },
        OperatorInfo {
            op_type: "Bitwise".into(),
            operator: "bit-xor".into(),
            name: "BitXor".into(),
            description: "Performs a bitwise XOR on two values.".into(),
            precedence: 70,
        },
        OperatorInfo {
            op_type: "Bitwise".into(),
            operator: "bit-and".into(),
            name: "BitAnd".into(),
            description: "Performs a bitwise AND on two values.".into(),
            precedence: 75,
        },
        OperatorInfo {
            op_type: "Bitwise".into(),
            operator: "bit-shl".into(),
            name: "ShiftLeft".into(),
            description: "Shifts a value left by another.".into(),
            precedence: 85,
        },
        OperatorInfo {
            op_type: "Bitwise".into(),
            operator: "bit-shr".into(),
            name: "ShiftRight".into(),
            description: "Shifts a value right by another.".into(),
            precedence: 85,
        },
        OperatorInfo {
            op_type: "Boolean".into(),
            operator: "and".into(),
            name: "And".into(),
            description: "Checks if two values are true.".into(),
            precedence: 50,
        },
        OperatorInfo {
            op_type: "Boolean".into(),
            operator: "or".into(),
            name: "Or".into(),
            description: "Checks if either value is true.".into(),
            precedence: 40,
        },
        OperatorInfo {
            op_type: "Boolean".into(),
            operator: "xor".into(),
            name: "Xor".into(),
            description: "Checks if one value is true and the other is false.".into(),
            precedence: 45,
        },
    ]
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
