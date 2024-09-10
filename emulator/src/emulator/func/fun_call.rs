
#[derive(Debug, Clone)]
pub struct FunctionCall {
    pub caller_address: i64,
    pub function_address: i64,
    pub return_address: i64,
    pub args: Vec<i64>,
}

impl FunctionCall {
    pub fn new(caller_address: i64, function_address: i64, return_address: i64, args: Vec<i64>) -> FunctionCall {
        FunctionCall {
            caller_address,
            function_address,
            return_address,
            args,
        }
    }
}