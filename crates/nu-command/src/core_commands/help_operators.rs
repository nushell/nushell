use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, IntoInterruptiblePipelineData, PipelineData, ShellError, Signature, Value,
};

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
        Signature::build("help operators").category(Category::Core)
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
            let mut cols = vec![];
            let mut vals = vec![];
            cols.push("type".into());
            vals.push(Value::string(op.op_type, head));
            cols.push("operator".into());
            vals.push(Value::string(op.operator, head));
            cols.push("name".into());
            vals.push(Value::string(op.name, head));
            cols.push("precedence".into());
            vals.push(Value::int(op.precedence, head));
            recs.push(Value::Record {
                cols,
                vals,
                span: head,
            })
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
    precedence: i64,
}

fn generate_operator_info() -> Vec<OperatorInfo> {
    vec![
        OperatorInfo {
            op_type: "Assignment".into(),
            operator: "=".into(),
            name: "Assign".into(),
            precedence: 10,
        },
        OperatorInfo {
            op_type: "Assignment".into(),
            operator: "+=".into(),
            name: "PlusAssign".into(),
            precedence: 10,
        },
        OperatorInfo {
            op_type: "Assignment".into(),
            operator: "-=".into(),
            name: "MinusAssign".into(),
            precedence: 10,
        },
        OperatorInfo {
            op_type: "Assignment".into(),
            operator: "*=".into(),
            name: "MultiplyAssign".into(),
            precedence: 10,
        },
        OperatorInfo {
            op_type: "Assignment".into(),
            operator: "/=".into(),
            name: "DivideAssign".into(),
            precedence: 10,
        },
        OperatorInfo {
            op_type: "Comparison".into(),
            operator: "==".into(),
            name: "Equal".into(),
            precedence: 80,
        },
        OperatorInfo {
            op_type: "Comparison".into(),
            operator: "!=".into(),
            name: "NotEqual".into(),
            precedence: 80,
        },
        OperatorInfo {
            op_type: "Comparison".into(),
            operator: "<".into(),
            name: "LessThan".into(),
            precedence: 80,
        },
        OperatorInfo {
            op_type: "Comparison".into(),
            operator: "<=".into(),
            name: "LessThanOrEqual".into(),
            precedence: 80,
        },
        OperatorInfo {
            op_type: "Comparison".into(),
            operator: ">".into(),
            name: "GreaterThan".into(),
            precedence: 80,
        },
        OperatorInfo {
            op_type: "Comparison".into(),
            operator: ">=".into(),
            name: "GreaterThanOrEqual".into(),
            precedence: 80,
        },
        OperatorInfo {
            op_type: "Comparison".into(),
            operator: "=~".into(),
            name: "RegexMatch".into(),
            precedence: 80,
        },
        OperatorInfo {
            op_type: "Comparison".into(),
            operator: "!~".into(),
            name: "NotRegexMatch".into(),
            precedence: 80,
        },
        OperatorInfo {
            op_type: "Comparison".into(),
            operator: "in".into(),
            name: "In".into(),
            precedence: 80,
        },
        OperatorInfo {
            op_type: "Comparison".into(),
            operator: "not-in".into(),
            name: "NotIn".into(),
            precedence: 80,
        },
        OperatorInfo {
            op_type: "Comparison".into(),
            operator: "starts-with".into(),
            name: "StartsWith".into(),
            precedence: 80,
        },
        OperatorInfo {
            op_type: "Comparison".into(),
            operator: "ends-with".into(),
            name: "EndsWith".into(),
            precedence: 80,
        },
        OperatorInfo {
            op_type: "Math".into(),
            operator: "+".into(),
            name: "Plus".into(),
            precedence: 90,
        },
        OperatorInfo {
            op_type: "Math".into(),
            operator: "++".into(),
            name: "Append".into(),
            precedence: 80,
        },
        OperatorInfo {
            op_type: "Math".into(),
            operator: "-".into(),
            name: "Minus".into(),
            precedence: 90,
        },
        OperatorInfo {
            op_type: "Math".into(),
            operator: "*".into(),
            name: "Multiply".into(),
            precedence: 95,
        },
        OperatorInfo {
            op_type: "Math".into(),
            operator: "/".into(),
            name: "Divide".into(),
            precedence: 95,
        },
        OperatorInfo {
            op_type: "Math".into(),
            operator: "//".into(),
            name: "FloorDivision".into(),
            precedence: 95,
        },
        OperatorInfo {
            op_type: "Math".into(),
            operator: "mod".into(),
            name: "Modulo".into(),
            precedence: 95,
        },
        OperatorInfo {
            op_type: "Math".into(),
            operator: "**".into(),
            name: "Pow ".into(),
            precedence: 100,
        },
        OperatorInfo {
            op_type: "Bitwise".into(),
            operator: "bit-or".into(),
            name: "BitOr".into(),
            precedence: 60,
        },
        OperatorInfo {
            op_type: "Bitwise".into(),
            operator: "bit-xor".into(),
            name: "BitXor".into(),
            precedence: 70,
        },
        OperatorInfo {
            op_type: "Bitwise".into(),
            operator: "bit-and".into(),
            name: "BitAnd".into(),
            precedence: 75,
        },
        OperatorInfo {
            op_type: "Bitwise".into(),
            operator: "bit-shl".into(),
            name: "ShiftLeft".into(),
            precedence: 85,
        },
        OperatorInfo {
            op_type: "Bitwise".into(),
            operator: "bit-shr".into(),
            name: "ShiftRight".into(),
            precedence: 85,
        },
        OperatorInfo {
            op_type: "Boolean".into(),
            operator: "&&".into(),
            name: "And".into(),
            precedence: 50,
        },
        OperatorInfo {
            op_type: "Boolean".into(),
            operator: "and".into(),
            name: "And".into(),
            precedence: 50,
        },
        OperatorInfo {
            op_type: "Boolean".into(),
            operator: "||".into(),
            name: "Or".into(),
            precedence: 40,
        },
        OperatorInfo {
            op_type: "Boolean".into(),
            operator: "or".into(),
            name: "Or".into(),
            precedence: 40,
        },
        OperatorInfo {
            op_type: "Boolean".into(),
            operator: "xor".into(),
            name: "Xor".into(),
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
