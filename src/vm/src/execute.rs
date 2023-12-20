use super::*;
use Code::*;

pub fn execute(code: &[Code], runtime: &mut Runtime) -> Result<Object, String> {
    let mut pc = 0;
    loop {
        // println!("code: {:?}", code[pc]);
        // runtime.dump();
        // println!();

        match &code[pc] {
            LoadInt(x) => {
                runtime.stack.push(Object::Int(*x).into());
            }
            LoadFloat(x) => {
                runtime.stack.push(Object::Float(*x).into());
            }
            LoadBool(x) => {
                runtime.stack.push(Object::Bool(*x).into());
            }
            LoadString(x) => {
                let x = StringObject::new(Rc::clone(x));
                runtime.stack.push(Object::String(x).into());
            }
            LoadNil => {
                runtime.stack.push(Object::Nil.into());
            }
            LoadLocal(id) => {
                let object = runtime.variable_table.get(*id);
                runtime.stack.push(object.into());
            }
            LoadRustFunction(x) => {
                runtime.stack.push(Object::RustFunction(*x).into());
            }
            UnloadTop => {
                runtime.stack.pop();
            }
            SetLocal(id) => {
                let object = runtime.stack.pop().ensure_object();
                runtime.variable_table.edit(*id, object);
            }
            MakeLocal => {
                let object = runtime.stack.pop().ensure_object();
                runtime.variable_table.push(object);
            }
            MakeArray(count) => {
                let mut array = Vec::with_capacity(*count as usize);
                for _ in 0..*count {
                    array.push(runtime.stack.pop().ensure_object());
                }
                array.reverse();
                runtime.stack.push(array.into());
            }
            MakeNamed => {
                let name = runtime.stack.pop().ensure_object().ensure_string()?;
                let object = runtime.stack.pop().ensure_object();
                runtime.stack.push((name, object).into());
            }
            MakeTable(count) => {
                let mut hash_map = HashMap::with_capacity(*count as usize);
                for _ in 0..*count {
                    let (name, value) = runtime.stack.pop().ensure_named();
                    let name = name.to_string();
                    hash_map.insert(name.into(), value);
                }
                let table = TableObject::new(hash_map);
                runtime.stack.push(Object::new_table(table).into());
            }
            DropLocal(count) => {
                runtime.variable_table.drop(*count);
            }
            Jump(offset) => {
                if offset.is_positive() {
                    pc += *offset as usize;
                } else {
                    pc -= offset.unsigned_abs();
                }
                continue;
            }
            JumpIfTrue(offset) => {
                let boolean = runtime.stack.pop().ensure_object().ensure_bool()?;
                if boolean {
                    if offset.is_positive() {
                        pc += *offset as usize;
                    } else {
                        pc -= offset.unsigned_abs();
                    }
                    continue;
                }
            }
            JumpIfFalse(offset) => {
                let boolean = runtime.stack.pop().ensure_object().ensure_bool()?;
                if !boolean {
                    if offset.is_positive() {
                        pc += *offset as usize;
                    } else {
                        pc -= offset.unsigned_abs();
                    }
                    continue;
                }
            }
            CallMethod(name, args_len) => {
                let mut rev_args = {
                    let mut args = Vec::with_capacity(*args_len as usize);
                    for _ in 0..*args_len {
                        args.push(runtime.stack.pop().ensure_object());
                    }
                    args
                };
                let self_obj = runtime.stack.pop().ensure_object();
                fn reversed(mut vec: Vec<Object>) -> Vec<Object> {
                    vec.reverse();
                    vec
                }
                match self_obj {
                    Object::Int(int) => {
                        let res = run_int_method(int, name, reversed(rev_args))?;
                        runtime.stack.push(res.into());
                    }
                    Object::Float(float) => {
                        let res = run_float_method(float, name, reversed(rev_args))?;
                        runtime.stack.push(res.into());
                    }
                    Object::String(string) => {
                        let res = run_string_method(string, name, reversed(rev_args))?;
                        runtime.stack.push(res.into());
                    }
                    Object::Bool(boolean) => {
                        let res = run_bool_method(boolean, name, reversed(rev_args))?;
                        runtime.stack.push(res.into());
                    }
                    Object::Nil => {
                        let res = run_nil_method(name, reversed(rev_args))?;
                        runtime.stack.push(res.into());
                    }
                    Object::Array(array) => {
                        let res = run_array_method(array, name, reversed(rev_args))?;
                        runtime.stack.push(res.into());
                    }
                    Object::Table(table) => {
                        let method = table.borrow().get_method(name);
                        let res = match method {
                            Some(TableMethod::Builtin(func)) => func(table, reversed(rev_args))?,
                            Some(TableMethod::Custom(func)) => {
                                rev_args.push(Object::Table(table));
                                execute_func(&func, reversed(rev_args), runtime)?
                            }
                            None => run_table_default_method(table, name, reversed(rev_args))?,
                        };
                        runtime.stack.push(res.into());
                    }
                    Object::Function(_) | Object::RustFunction(_) => {
                        Err("Function does not have methods.".to_string())?
                    }
                }
            }
            Call(args_len) => {
                let args = create_args_vec(*args_len, runtime);
                let ret = match runtime.stack.pop() {
                    StackValue::RawFunction(func) => execute_func(&func, args, runtime)?,
                    StackValue::Object(Object::Function(func)) => {
                        execute_func(&func, args, runtime)?
                    }
                    StackValue::Object(Object::Table(table)) => {
                        let method = table.borrow().get_method("__call");
                        match method {
                            Some(TableMethod::Builtin(func)) => func(table, args)?,
                            Some(TableMethod::Custom(func)) => execute_func(&func, args, runtime)?,
                            None => Err("__call is not defined.".to_string())?,
                        }
                    }
                    StackValue::Object(Object::RustFunction(func)) => func(&args)?,
                    x => Err(format!("Expected Callable Object, but got {:?}", x))?,
                };
                runtime.stack.push(ret.into());
            }
            SetItem => {
                let accesser = runtime.stack.pop().ensure_object();
                let target = runtime.stack.pop();
                let value = runtime.stack.pop().ensure_object();
                match target {
                    StackValue::RawArray(mut array) => {
                        let index = accesser.ensure_int()?;
                        array[index as usize] = value;
                        runtime.stack.push(array.into());
                    }
                    StackValue::Object(Object::Array(array)) => {
                        let index = accesser.ensure_int()?;
                        array.borrow_mut()[index as usize] = value;
                        runtime.stack.push(Object::Array(array).into());
                    }
                    StackValue::Object(Object::Table(table)) => {
                        let index = accesser.ensure_string()?;
                        if let Some(t) = table.borrow_mut().get_mut(index.as_str()) {
                            *t = value;
                        } else {
                            let index = index.to_string();
                            table.borrow_mut().insert(index.into(), value);
                        }
                        runtime.stack.push(Object::Table(table).into());
                    }
                    x => Err(format!("Expected Array or Table, but got {:?}", x))?,
                }
            }
            GetItem => {
                let accesser = runtime.stack.pop().ensure_object();
                let target = runtime.stack.pop();
                match target {
                    StackValue::RawArray(array) => {
                        let index = accesser.ensure_int()?;
                        let item = match array.get(index as usize) {
                            Some(x) => x.clone(),
                            None => Object::Nil,
                        };
                        runtime.stack.push(item.into());
                    }
                    StackValue::Object(Object::String(string)) => {
                        let string = string.get_chars();
                        let index = {
                            let i = accesser.ensure_int()?;
                            if i >= 0 {
                                string.len() as i64 + i
                            } else {
                                i
                            }
                        };
                        let item = match string.get(index as usize) {
                            Some(x) => Object::new_string(x.to_string()),
                            None => Object::Nil,
                        };
                        runtime.stack.push(item.into());
                    }
                    StackValue::Object(Object::Array(array)) => {
                        let index = accesser.ensure_int()?;
                        let item = match array.borrow().get(index as usize) {
                            Some(x) => x.clone(),
                            None => Object::Nil,
                        };
                        runtime.stack.push(item.into());
                    }
                    StackValue::Object(Object::Table(table)) => {
                        let index = accesser.ensure_string()?;
                        let item = match table.borrow().get(index.as_str()) {
                            Some(x) => x.clone(),
                            None => Object::Nil,
                        };
                        runtime.stack.push(item.into());
                    }
                    x => Err(format!("Expected Array or Table, but got {:?}", x))?,
                }
            }
            Add => {
                let rhs = runtime.stack.pop().ensure_object();
                let lhs = runtime.stack.pop().ensure_object();
                match (lhs, rhs) {
                    (Object::Int(lhs), Object::Int(rhs)) => {
                        runtime.stack.push(Object::Int(lhs + rhs).into());
                    }
                    (Object::Int(lhs), Object::Float(rhs)) => {
                        runtime.stack.push(Object::Float(lhs as f64 + rhs).into());
                    }
                    (Object::Float(lhs), Object::Int(rhs)) => {
                        runtime.stack.push(Object::Float(lhs + rhs as f64).into());
                    }
                    (Object::Float(lhs), Object::Float(rhs)) => {
                        runtime.stack.push(Object::Float(lhs + rhs).into());
                    }
                    (lhs, rhs) => Err(format!(
                        "Expected Int or Float, but got {:?} and {:?}",
                        lhs, rhs
                    ))?,
                }
            }
            Sub => {
                let rhs = runtime.stack.pop().ensure_object();
                let lhs = runtime.stack.pop().ensure_object();
                match (lhs, rhs) {
                    (Object::Int(lhs), Object::Int(rhs)) => {
                        runtime.stack.push(Object::Int(lhs - rhs).into());
                    }
                    (Object::Int(lhs), Object::Float(rhs)) => {
                        runtime.stack.push(Object::Float(lhs as f64 - rhs).into());
                    }
                    (Object::Float(lhs), Object::Int(rhs)) => {
                        runtime.stack.push(Object::Float(lhs - rhs as f64).into());
                    }
                    (Object::Float(lhs), Object::Float(rhs)) => {
                        runtime.stack.push(Object::Float(lhs - rhs).into());
                    }
                    (lhs, rhs) => Err(format!(
                        "Expected Int or Float, but got {:?} and {:?}",
                        lhs, rhs
                    ))?,
                }
            }
            Mul => {
                let rhs = runtime.stack.pop().ensure_object();
                let lhs = runtime.stack.pop().ensure_object();
                match (lhs, rhs) {
                    (Object::Int(lhs), Object::Int(rhs)) => {
                        runtime.stack.push(Object::Int(lhs * rhs).into());
                    }
                    (Object::Int(lhs), Object::Float(rhs)) => {
                        runtime.stack.push(Object::Float(lhs as f64 * rhs).into());
                    }
                    (Object::Float(lhs), Object::Int(rhs)) => {
                        runtime.stack.push(Object::Float(lhs * rhs as f64).into());
                    }
                    (Object::Float(lhs), Object::Float(rhs)) => {
                        runtime.stack.push(Object::Float(lhs * rhs).into());
                    }
                    (lhs, rhs) => Err(format!(
                        "Expected Int or Float, but got {:?} and {:?}",
                        lhs, rhs
                    ))?,
                }
            }
            Div => {
                let rhs = runtime.stack.pop().ensure_object();
                let lhs = runtime.stack.pop().ensure_object();
                match (lhs, rhs) {
                    (Object::Int(lhs), Object::Int(rhs)) => {
                        runtime.stack.push(Object::Int(lhs / rhs).into());
                    }
                    (Object::Int(lhs), Object::Float(rhs)) => {
                        runtime.stack.push(Object::Float(lhs as f64 / rhs).into());
                    }
                    (Object::Float(lhs), Object::Int(rhs)) => {
                        runtime.stack.push(Object::Float(lhs / rhs as f64).into());
                    }
                    (Object::Float(lhs), Object::Float(rhs)) => {
                        runtime.stack.push(Object::Float(lhs / rhs).into());
                    }
                    (lhs, rhs) => Err(format!(
                        "Expected Int or Float, but got {:?} and {:?}",
                        lhs, rhs
                    ))?,
                }
            }
            Mod => {
                let rhs = runtime.stack.pop().ensure_object();
                let lhs = runtime.stack.pop().ensure_object();
                match (lhs, rhs) {
                    (Object::Int(lhs), Object::Int(rhs)) => {
                        runtime.stack.push(Object::Int(lhs % rhs).into());
                    }
                    (Object::Int(lhs), Object::Float(rhs)) => {
                        runtime.stack.push(Object::Float(lhs as f64 % rhs).into());
                    }
                    (Object::Float(lhs), Object::Int(rhs)) => {
                        runtime.stack.push(Object::Float(lhs % rhs as f64).into());
                    }
                    (Object::Float(lhs), Object::Float(rhs)) => {
                        runtime.stack.push(Object::Float(lhs % rhs).into());
                    }
                    (lhs, rhs) => Err(format!(
                        "Expected Int or Float, but got {:?} and {:?}",
                        lhs, rhs
                    ))?,
                }
            }
            Pow => {
                let rhs = runtime.stack.pop().ensure_object();
                let lhs = runtime.stack.pop().ensure_object();
                match (lhs, rhs) {
                    (Object::Int(_lhs), Object::Int(_rhs)) => {
                        unimplemented!("Int.pow(Int) is not implemented.");
                    }
                    (Object::Int(lhs), Object::Float(rhs)) => {
                        let pow = (lhs as f64).powf(rhs);
                        runtime.stack.push(Object::Float(pow).into());
                    }
                    (Object::Float(lhs), Object::Int(rhs)) => {
                        let pow = if rhs > i32::MAX as i64 {
                            lhs.powf(rhs as f64)
                        } else {
                            lhs.powi(rhs as i32)
                        };
                        runtime.stack.push(Object::Float(pow).into());
                    }
                    (Object::Float(lhs), Object::Float(rhs)) => {
                        let pow = lhs.powf(rhs);
                        runtime.stack.push(Object::Float(pow).into());
                    }
                    (lhs, rhs) => Err(format!(
                        "Expected Int or Float, but got {:?} and {:?}",
                        lhs, rhs
                    ))?,
                }
            }
            Unm => {
                let obj = runtime.stack.pop().ensure_object();
                match obj {
                    Object::Int(x) => runtime.stack.push(Object::Int(-x).into()),
                    Object::Float(x) => runtime.stack.push(Object::Float(-x).into()),
                    x => Err(format!("Expected Int or Float, but got {:?}", x))?,
                }
            }
            Eq => {
                let rhs = runtime.stack.pop().ensure_object();
                let lhs = runtime.stack.pop().ensure_object();
                runtime.stack.push(Object::Bool(lhs == rhs).into());
            }
            NotEq => {
                let rhs = runtime.stack.pop().ensure_object();
                let lhs = runtime.stack.pop().ensure_object();
                runtime.stack.push(Object::Bool(lhs != rhs).into());
            }
            Less => {
                let rhs = runtime.stack.pop().ensure_object();
                let lhs = runtime.stack.pop().ensure_object();
                match (lhs, rhs) {
                    (Object::Int(lhs), Object::Int(rhs)) => {
                        runtime.stack.push(Object::Bool(lhs < rhs).into());
                    }
                    (Object::Int(lhs), Object::Float(rhs)) => {
                        runtime.stack.push(Object::Bool((lhs as f64) < rhs).into());
                    }
                    (Object::Float(lhs), Object::Int(rhs)) => {
                        runtime.stack.push(Object::Bool(lhs < (rhs as f64)).into());
                    }
                    (Object::Float(lhs), Object::Float(rhs)) => {
                        runtime.stack.push(Object::Bool(lhs < rhs).into());
                    }
                    (lhs, rhs) => Err(format!(
                        "Expected Int or Float, but got {:?} and {:?}",
                        lhs, rhs
                    ))?,
                }
            }
            LessEq => {
                let rhs = runtime.stack.pop().ensure_object();
                let lhs = runtime.stack.pop().ensure_object();
                match (lhs, rhs) {
                    (Object::Int(lhs), Object::Int(rhs)) => {
                        runtime.stack.push(Object::Bool(lhs <= rhs).into());
                    }
                    (Object::Int(lhs), Object::Float(rhs)) => {
                        runtime.stack.push(Object::Bool((lhs as f64) <= rhs).into());
                    }
                    (Object::Float(lhs), Object::Int(rhs)) => {
                        runtime.stack.push(Object::Bool(lhs <= (rhs as f64)).into());
                    }
                    (Object::Float(lhs), Object::Float(rhs)) => {
                        runtime.stack.push(Object::Bool(lhs <= rhs).into());
                    }
                    (lhs, rhs) => Err(format!(
                        "Expected Int or Float, but got {:?} and {:?}",
                        lhs, rhs
                    ))?,
                }
            }
            Greater => {
                let rhs = runtime.stack.pop().ensure_object();
                let lhs = runtime.stack.pop().ensure_object();
                match (lhs, rhs) {
                    (Object::Int(lhs), Object::Int(rhs)) => {
                        runtime.stack.push(Object::Bool(lhs > rhs).into());
                    }
                    (Object::Int(lhs), Object::Float(rhs)) => {
                        runtime.stack.push(Object::Bool((lhs as f64) > rhs).into());
                    }
                    (Object::Float(lhs), Object::Int(rhs)) => {
                        runtime.stack.push(Object::Bool(lhs > (rhs as f64)).into());
                    }
                    (Object::Float(lhs), Object::Float(rhs)) => {
                        runtime.stack.push(Object::Bool(lhs > rhs).into());
                    }
                    (lhs, rhs) => Err(format!(
                        "Expected Int or Float, but got {:?} and {:?}",
                        lhs, rhs
                    ))?,
                }
            }
            GreaterEq => {
                let rhs = runtime.stack.pop().ensure_object();
                let lhs = runtime.stack.pop().ensure_object();
                match (lhs, rhs) {
                    (Object::Int(lhs), Object::Int(rhs)) => {
                        runtime.stack.push(Object::Bool(lhs >= rhs).into());
                    }
                    (Object::Int(lhs), Object::Float(rhs)) => {
                        runtime.stack.push(Object::Bool((lhs as f64) >= rhs).into());
                    }
                    (Object::Float(lhs), Object::Int(rhs)) => {
                        runtime.stack.push(Object::Bool(lhs >= (rhs as f64)).into());
                    }
                    (Object::Float(lhs), Object::Float(rhs)) => {
                        runtime.stack.push(Object::Bool(lhs >= rhs).into());
                    }
                    (lhs, rhs) => Err(format!(
                        "Expected Int or Float, but got {:?} and {:?}",
                        lhs, rhs
                    ))?,
                }
            }
            Concat => {
                let rhs = runtime.stack.pop().ensure_object();
                let lhs = runtime.stack.pop().ensure_object();
                // TODO: Improve performance when lhs or rhs is Object::String.
                fn to_string(obj: Object) -> Result<String, String> {
                    match obj {
                        Object::Int(x) => Ok(x.to_string()),
                        Object::Float(x) => Ok(x.to_string()),
                        Object::String(x) => Ok(x.to_string()),
                        Object::Bool(x) => Ok(if x { "true" } else { "false" }.to_string()),
                        Object::Nil => Ok("nil".to_string()),
                        x => Err(format!(
                            "Expected String or Stringable Object, but got {:?}",
                            x
                        ))?,
                    }
                }
                let lhs = to_string(lhs)?;
                let rhs = to_string(rhs)?;
                runtime.stack.push(Object::new_string(lhs + &rhs).into());
            }
            Builtin(instr, args_len) => {
                let args = create_args_vec(*args_len, runtime);
                match instr {
                    BuiltinInstr::Write => {
                        for arg in args {
                            runtime.stdio.write(format!("{}", arg));
                        }
                    }
                    BuiltinInstr::Flush => {
                        assert!(*args_len == 0, "Builtin::Flush takes no arguments.");
                        runtime.stdio.flush();
                    }
                    BuiltinInstr::WriteError => {
                        for arg in args {
                            runtime.stdio.write_err(format!("{}", arg));
                        }
                    }
                    BuiltinInstr::FlushError => {
                        assert!(*args_len == 0, "Builtin::FlushError takes no arguments.");
                        runtime.stdio.flush_err();
                    }
                    BuiltinInstr::ReadLine => {
                        assert!(*args_len == 0, "Builtin::ReadLine takes no arguments.");
                        let line = runtime.stdio.read_line();
                        runtime.stack.push(Object::new_string(line).into());
                    }
                    BuiltinInstr::ReadFile => {
                        assert!(*args_len == 1, "Builtin::ReadFile takes 1 argument.");
                        let path = args.into_iter().next().unwrap().ensure_string()?;
                        let content = std::fs::read(path.as_str()).map_err(|e| e.to_string())?;
                        let string = String::from_utf8(content).map_err(|e| e.to_string())?;
                        runtime.stack.push(Object::new_string(string).into());
                    }
                    BuiltinInstr::WriteFile => {
                        assert!(*args_len == 2, "Builtin::WriteFile takes 2 arguments.");
                        let mut args = args.into_iter();
                        let path = args.next().unwrap().ensure_string()?;
                        let content = args.next().unwrap().ensure_string()?;
                        std::fs::write(path.as_str(), content.as_str())
                            .map_err(|e| e.to_string())?;
                    }
                }
            }
            BeginFuncCreation => {
                let id = (pc, 0u8);
                pc += 1;
                let env = {
                    let mut env = Vec::new();
                    while let AddCapture(name) = code[pc] {
                        let obj = runtime.variable_table.get_ref(name);
                        env.push(obj);
                        pc += 1;
                    }
                    env
                };
                let args = {
                    let mut args = Vec::new();
                    while let AddArgument(name) = code[pc] {
                        args.push(name);
                        pc += 1;
                    }
                    args
                };
                let code = {
                    let mut func_code = Vec::new();
                    let mut inner_count = 0;
                    loop {
                        if let BeginFuncCreation = code[pc] {
                            inner_count += 1;
                        } else if let EndFuncCreation = code[pc] {
                            inner_count -= 1;
                        }
                        if inner_count < 0 {
                            break;
                        }
                        func_code.push(code[pc].clone());
                        pc += 1;
                    }
                    func_code
                };
                runtime.stack.push(
                    Object::new_function(FunctionObject {
                        id,
                        env,
                        args,
                        code,
                    })
                    .into(),
                );
            }
            AddCapture(_) => panic!("[BUG] AddCapture is not allowed here."),
            AddArgument(_) => panic!("[BUG] AddArgument is not allowed here."),
            EndFuncCreation => panic!("[BUG] EndFuncCreation is not allowed here."),
            Nop => {}
            Return => {
                return Ok(runtime.stack.pop().ensure_object());
            }
            Exit => {
                return Ok(Object::Nil);
            }
        }
        pc += 1;
    }
}

fn execute_func(
    func: &FunctionObject,
    args: Vec<Object>,
    runtime: &mut Runtime,
) -> Result<Object, String> {
    if func.args.len() != args.len() {
        return Err(format!(
            "Expected {} arguments, but got {} arguments.",
            func.args.len(),
            args.len()
        ));
    }
    runtime.variable_table.push_scope();
    for value in func.env.iter() {
        runtime.variable_table.push_ref(Rc::clone(value));
    }
    for (_attr, value) in func.args.iter().zip(args.iter()) {
        runtime.variable_table.push(value.clone());
    }
    let ret = execute(&func.code, runtime)?;
    runtime.variable_table.pop_scope();
    Ok(ret)
}

fn create_args_vec(args_len: u8, runtime: &mut Runtime) -> Vec<Object> {
    let mut args = Vec::with_capacity(args_len as usize);
    for _ in 0..args_len {
        args.push(runtime.stack.pop().ensure_object());
    }
    args.reverse();
    args
}