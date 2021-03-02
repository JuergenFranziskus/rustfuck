use crate::front_end::parser::{InstructionNode, NodeType};
use inkwell::context::{Context, ContextRef};
use inkwell::module::Module;
use inkwell::builder::Builder;
use inkwell::values::{FunctionValue, PointerValue, GlobalValue, BasicValueEnum, IntValue, InstructionOpcode, AnyValue};
use inkwell::{AddressSpace, IntPredicate};
use inkwell::targets::TargetData;
use inkwell::AtomicRMWBinOp::Add;
use inkwell::memory_buffer::MemoryBuffer;
use inkwell::basic_block::BasicBlock;


pub fn compile_to_ir(node: &InstructionNode, module_name: &str) -> String {
    let context = Context::create();
    let ctx = CompilationContext::new(module_name, &context);
    let symbols = Symbols::new(&ctx);

    let entry = build_entry_block(&ctx, &symbols);
    let vars = build_variables(&ctx, &symbols);
    init_variables(&ctx, &symbols, &vars, entry);

    build_node(&ctx, &symbols, &vars, node);

    free_variables(&ctx, &symbols, &vars);
    exit_program(&ctx, &symbols);

    ctx.module.print_to_string().to_string()
}

fn build_entry_block<'ctx>(ctx: &CompilationContext<'ctx>, symbols: &Symbols) -> BasicBlock<'ctx>{
    let entry = ctx.context.append_basic_block(symbols.start, "entry");
    ctx.builder.position_at_end(entry);
    entry
}
fn build_variables<'ctx>(ctx: &CompilationContext<'ctx>, symbols: &Symbols) -> Variables<'ctx> {
    Variables::new(ctx, symbols)
}
fn init_variables(ctx: &CompilationContext, symbols: &Symbols, vars: &Variables, entry: BasicBlock) {
    ctx.builder.position_at_end(entry);

    let val_30000 = ctx.context.i64_type().const_int(30000, false);
    let val_0 = ctx.context.i64_type().const_int(0, false);
    let val_0_32 = ctx.context.i32_type().const_int(0, false);
    let i8_ptr_type = ctx.context.i8_type().ptr_type(AddressSpace::Generic);

    ctx.builder.build_store(vars.len, val_30000);
    ctx.builder.build_store(vars.index, val_0);

    let success = ctx.context.append_basic_block(symbols.start, "alloc_success");
    let failed =  ctx.context.append_basic_block(symbols.start, "alloc_failed");



    let alloc_result = ctx.builder.build_call(symbols.malloc, &[val_30000.into()], "alloc_result");
    let alloc_ret_val = alloc_result.as_any_value_enum().into_pointer_value();

    let is_nullptr = ctx.builder.build_is_null(alloc_ret_val, "is_nullptr");
    ctx.builder.build_conditional_branch(is_nullptr, failed, success);

    ctx.builder.position_at_end(failed);
    let error_msg = ctx.builder.build_pointer_cast(symbols.alloc_failed.as_pointer_value(), i8_ptr_type, "err_msg");
    ctx.builder.build_call(symbols.puts, &[error_msg.into()], "");
    exit_program(ctx, symbols);


    ctx.builder.position_at_end(success);
    ctx.builder.build_store(vars.array, alloc_ret_val);
    ctx.builder.build_call(symbols.memset, &[alloc_ret_val.into(), val_0_32.into(), val_30000.into()], "");
}

fn build_node(ctx: &CompilationContext, symbols: &Symbols, vars: &Variables, node: &InstructionNode) {
    match &node.node_type {
        NodeType::Program(children) => {
            for child in children {
                build_node(ctx, symbols, vars, child);
            }
        }
        NodeType::Loop(children) => build_loop(ctx, symbols, vars, children),
        NodeType::Next(amount) => build_next(ctx, symbols, vars, *amount),
        NodeType::Previous(amount) => build_previous(ctx, symbols, vars, *amount),
        NodeType::Increment(amount) => build_increment(ctx, symbols, vars, *amount),
        NodeType::Decrement(amount) => build_decrement(ctx, symbols, vars, *amount),
        NodeType::Output => build_output(ctx, symbols, vars),
        NodeType::Input => build_input(ctx, symbols, vars),
        NodeType::SetCell(value) => build_set(ctx, symbols, vars, *value),
    }
}
fn build_loop(ctx: &CompilationContext, symbols: &Symbols, vars: &Variables, children: &Vec<InstructionNode>) {
    let loop_header = ctx.context.append_basic_block(symbols.start, "loop_header");
    let loop_body = ctx.context.append_basic_block(symbols.start, "loop_body");
    let loop_end = ctx.context.append_basic_block(symbols.start, "loop_end");

    ctx.builder.build_unconditional_branch(loop_header);

    ctx.builder.position_at_end(loop_header);
    build_resize(ctx, symbols, vars);
    let i_val = ctx.builder.build_load(vars.index, "index_val");
    let arr_ptr = ctx.builder.build_load(vars.array, "arr_ptr");
    let cell_ptr = unsafe { ctx.builder.build_gep(arr_ptr.into_pointer_value(), &[i_val.into_int_value()], "cell_ptr") };
    let cell_val = ctx.builder.build_load(cell_ptr, "cell_val");
    let val_0 = ctx.context.i8_type().const_int(0, false);
    let is_zero = ctx.builder.build_int_compare(IntPredicate::EQ, cell_val.into_int_value(), val_0.into(), "is_zero");
    ctx.builder.build_conditional_branch(is_zero, loop_end, loop_body);


    ctx.builder.position_at_end(loop_body);
    for child in children {
        build_node(ctx, symbols, vars, child);
    }
    ctx.builder.build_unconditional_branch(loop_header);


    ctx.builder.position_at_end(loop_end);
}
fn build_next(ctx: &CompilationContext, symbols: &Symbols, vars: &Variables, amount: usize) {
    let amount_val = ctx.context.i64_type().const_int(amount as u64, false);
    ctx.builder.build_call(symbols.next(), &[vars.index.into(), amount_val.into()], "");
}
fn build_previous(ctx: &CompilationContext, symbols: &Symbols, vars: &Variables, amount: usize) {
    let amount_val = ctx.context.i64_type().const_int(amount as u64, false);
    ctx.builder.build_call(symbols.previous(), &[vars.array.into(), vars.len.into(), vars.index.into(), amount_val.into()], "");
}
fn build_increment(ctx: &CompilationContext, symbols: &Symbols, vars: &Variables, amount: usize) {
    let amount_val = ctx.context.i8_type().const_int(amount as u64 % 255, false);
    let args: [BasicValueEnum; 4] = [vars.array.into(), vars.len.into(), vars.index.into(), amount_val.into()];
    ctx.builder.build_call(symbols.increment(), &args, "");
}
fn build_decrement(ctx: &CompilationContext, symbols: &Symbols, vars: &Variables, amount: usize) {
    let amount_val = ctx.context.i8_type().const_int(amount as u64 % 255, false);
    let args: [BasicValueEnum; 4] = [vars.array.into(), vars.len.into(), vars.index.into(), amount_val.into()];
    ctx.builder.build_call(symbols.decrement(), &args, "");
}
fn build_output(ctx: &CompilationContext, symbols: &Symbols, vars: &Variables) {
    let args: [BasicValueEnum; 3] = [vars.array.into(), vars.len.into(), vars.index.into()];
    ctx.builder.build_call(symbols.output(), &args, "");
}
fn build_input(ctx: &CompilationContext, symbols: &Symbols, vars: &Variables) {
    let args: [BasicValueEnum; 3] = [vars.array.into(), vars.len.into(), vars.index.into()];
    ctx.builder.build_call(symbols.input(), &args, "");
}
fn build_set(ctx: &CompilationContext, symbols: &Symbols, vars: &Variables, value: usize) {
    let value_val = ctx.context.i8_type().const_int(value as u64 % 255, false);
    let args: [BasicValueEnum; 4] = [vars.array.into(), vars.len.into(), vars.index.into(), value_val.into()];
    ctx.builder.build_call(symbols.set(), &args, "");
}

fn build_resize(ctx: &CompilationContext, symbols: &Symbols, vars: &Variables) {
    ctx.builder.build_call(symbols.resize(), &[vars.array.into(), vars.len.into(), vars.index.into()], "");
}


fn free_variables(ctx: &CompilationContext, symbols: &Symbols, vars: &Variables) {
    let arr_ptr = ctx.builder.build_load(vars.array, "arr_ptr");
    ctx.builder.build_call(symbols.free, &[arr_ptr.into()], "");
}
fn exit_program(ctx: &CompilationContext, symbols: &Symbols) {
    let val_0_32 = ctx.context.i32_type().const_int(0, false);
    ctx.builder.build_call(symbols.exit, &[val_0_32.into()], "");
    ctx.builder.build_return(None);
}


struct CompilationContext<'ctx> {
    context: &'ctx Context,
    module: Module<'ctx>,
    builder: Builder<'ctx>,
}
impl<'ctx> CompilationContext<'ctx> {
    pub fn new(module_name: &str, context: &'ctx Context) -> CompilationContext<'ctx> {
        let module = context.create_module(module_name);
        let builder = context.create_builder();

        CompilationContext {
            context,
            module,
            builder,
        }
    }
}

struct Symbols<'ctx> {
    start: FunctionValue<'ctx>,

    malloc: FunctionValue<'ctx>,
    free: FunctionValue<'ctx>,
    putchar: FunctionValue<'ctx>,
    getchar: FunctionValue<'ctx>,
    puts: FunctionValue<'ctx>,
    exit: FunctionValue<'ctx>,
    memset: FunctionValue<'ctx>,
    memcpy: FunctionValue<'ctx>,
    flush_stdout: FunctionValue<'ctx>,

    alloc_failed: GlobalValue<'ctx>,
    resize_failed: GlobalValue<'ctx>,
    index_underflow: GlobalValue<'ctx>,


    resize: Option<FunctionValue<'ctx>>,
    next: Option<FunctionValue<'ctx>>,
    previous: Option<FunctionValue<'ctx>>,
    increment: Option<FunctionValue<'ctx>>,
    decrement: Option<FunctionValue<'ctx>>,
    output: Option<FunctionValue<'ctx>>,
    input: Option<FunctionValue<'ctx>>,
    set: Option<FunctionValue<'ctx>>,
}
impl<'ctx> Symbols<'ctx> {
    pub fn new(ctx: &CompilationContext<'ctx>) -> Symbols<'ctx> {
        let start = Self::build_start_function(ctx);
        let malloc = Self::build_malloc_function(ctx);
        let free = Self::build_free_function(ctx);
        let putchar = Self::build_putchar_function(ctx);
        let getchar = Self::build_getchar_function(ctx);
        let puts = Self::build_puts(ctx);
        let exit = Self::build_exit(ctx);
        let memset = Self::build_memset(ctx);
        let memcpy = Self::build_memcpy(ctx);
        let flush_stdout = Self::build_flush_stdout(ctx);

        let alloc_failed = Self::build_const_str(ctx, "\nError: Failed to allocate cell array\n", "alloc_failed");
        let resize_failed = Self::build_const_str(ctx, "\nError: Failed to resize cell array\n", "resize_failed");
        let index_underflow = Self::build_const_str(ctx, "\nError: Tried to decrement index, resulting underflow\n", "index_underflow");

        let mut symbols = Symbols {
            start,

            malloc,
            free,
            putchar,
            getchar,
            puts,
            exit,
            memset,
            memcpy,
            flush_stdout,

            alloc_failed,
            resize_failed,
            index_underflow,

            resize: None,
            next: None,
            previous: None,
            increment: None,
            decrement: None,
            output: None,
            input: None,
            set: None,
        };
        symbols.build_resize(ctx);
        symbols.build_next(ctx);
        symbols.build_previous(ctx);
        symbols.build_increment(ctx);
        symbols.build_decrement(ctx);
        symbols.build_output(ctx);
        symbols.build_input(ctx);
        symbols.build_set(ctx);

        symbols
    }

    pub fn resize(&self) -> FunctionValue {
        self.resize.unwrap()
    }
    pub fn next(&self) -> FunctionValue {
        self.next.unwrap()
    }
    pub fn previous(&self) -> FunctionValue {
        self.previous.unwrap()
    }
    pub fn increment(&self) -> FunctionValue {
        self.increment.unwrap()
    }
    pub fn decrement(&self) -> FunctionValue {
        self.decrement.unwrap()
    }
    pub fn output(&self) -> FunctionValue {
        self.output.unwrap()
    }
    pub fn input(&self) -> FunctionValue {
        self.input.unwrap()
    }
    pub fn set(&self) -> FunctionValue {
        self.set.unwrap()
    }

    fn build_start_function(ctx: &CompilationContext<'ctx>) -> FunctionValue<'ctx> {
        let void_t = ctx.context.void_type();
        let fn_type = void_t.fn_type(&[], false);

        let start = ctx.module.add_function("_start", fn_type, None);

        start
    }
    fn build_malloc_function(context: &CompilationContext<'ctx>) -> FunctionValue<'ctx> {
        let ret_t = context.context.i8_type().ptr_type(AddressSpace::Generic);
        let size_t = context.context.i64_type();

        let fn_type = ret_t.fn_type(&[size_t.into()], false);
        let function = context.module.add_function("malloc", fn_type, None);

        function
    }
    fn build_free_function(context: &CompilationContext<'ctx>) -> FunctionValue<'ctx> {
        let void_t = context.context.void_type();
        let i8_ptr_t = context.context.i8_type().ptr_type(AddressSpace::Generic);

        let fn_type = void_t.fn_type(&[i8_ptr_t.into()], false);
        let function = context.module.add_function("free", fn_type, None);

        function
    }
    fn build_putchar_function(context: &CompilationContext<'ctx>) -> FunctionValue<'ctx> {
        let i32_t = context.context.i32_type();

        let fn_type = i32_t.fn_type(&[i32_t.into()], false);
        let function = context.module.add_function("putchar", fn_type, None);
        function
    }
    fn build_getchar_function(context: &CompilationContext<'ctx>) -> FunctionValue<'ctx> {
        let i32_t = context.context.i32_type();

        let fn_type = i32_t.fn_type(&[], false);
        let function = context.module.add_function("getchar", fn_type, None);

        function
    }
    fn build_puts(context: &CompilationContext<'ctx>) -> FunctionValue<'ctx> {
        let ret_t = context.context.i32_type();
        let i8_ptr_t = context.context.i8_type().ptr_type(AddressSpace::Generic);

        let fn_type = ret_t.fn_type(&[i8_ptr_t.into()], false);
        let function = context.module.add_function("puts", fn_type, None);


        function
    }
    fn build_exit(context: &CompilationContext<'ctx>) -> FunctionValue<'ctx> {
        let void_t = context.context.void_type();
        let i32_t = context.context.i32_type();

        let fn_type = void_t.fn_type(&[i32_t.into()], false);
        let function = context.module.add_function("exit", fn_type, None);

        function
    }
    fn build_memset(ctx: &CompilationContext<'ctx>) -> FunctionValue<'ctx> {
        let i8_ptr_t = ctx.context.i8_type().ptr_type(AddressSpace::Generic);
        let i32_t = ctx.context.i32_type();
        let i64_t = ctx.context.i64_type();

        let fn_type = i8_ptr_t.fn_type(&[i8_ptr_t.into(), i32_t.into(), i64_t.into()], false);
        let function = ctx.module.add_function("memset", fn_type, None);

        function
    }
    fn build_memcpy(ctx: &CompilationContext<'ctx>) -> FunctionValue<'ctx> {
        let i8_ptr_t = ctx.context.i8_type().ptr_type(AddressSpace::Generic);
        let i64_t = ctx.context.i64_type();

        let fn_type = i8_ptr_t.fn_type(&[i8_ptr_t.into(), i8_ptr_t.into(), i64_t.into()], false);
        let function = ctx.module.add_function("memcpy", fn_type, None);

        function
    }
    fn build_flush_stdout(ctx: &CompilationContext<'ctx>) -> FunctionValue<'ctx> {
        let void_t = ctx.context.void_type();
        let fn_type = void_t.fn_type(&[], false);
        let function = ctx.module.add_function("flush_stdout", fn_type, None);

        function
    }

    fn build_resize(&mut self, ctx: &CompilationContext<'ctx>) {
        let i8_ptr_ptr_t = ctx.context.i8_type().ptr_type(AddressSpace::Generic).ptr_type(AddressSpace::Generic);
        let i64_ptr_t = ctx.context.i64_type().ptr_type(AddressSpace::Generic);
        let void_t = ctx.context.void_type();

        let fn_type = void_t.fn_type(&[i8_ptr_ptr_t.into(), i64_ptr_t.into(), i64_ptr_t.into()], false);
        let resize = ctx.module.add_function("resize", fn_type, None);

        let entry = ctx.context.append_basic_block(resize, "entry");
        ctx.builder.position_at_end(entry);

        let array = resize.get_nth_param(0).unwrap().into_pointer_value();
        let len = resize.get_nth_param(1).unwrap().into_pointer_value();
        let index = resize.get_nth_param(2).unwrap().into_pointer_value();



        let do_resize = ctx.context.append_basic_block(resize, "do_resize");
        let resized = ctx.context.append_basic_block(resize, "resized");
        let alloc_failed = ctx.context.append_basic_block(resize, "resize_failed");
        let alloc_success = ctx.context.append_basic_block(resize, "alloc_success");


        let len_val = ctx.builder.build_load(len, "len_val");
        let i_val = ctx.builder.build_load(index, "index_val");
        let needs = ctx.builder.build_int_compare(IntPredicate::UGE, i_val.into_int_value(), len_val.into_int_value(), "needs_resize");
        ctx.builder.build_conditional_branch(needs, do_resize, resized);


        ctx.builder.position_at_end(do_resize);
        let val_100 = ctx.context.i64_type().const_int(100, false);
        let new_len = ctx.builder.build_int_add(i_val.into_int_value(), val_100.into(), "new_len");
        let alloc_result = ctx.builder.build_call(self.malloc, &[new_len.into()], "new_arr");
        let new_arr = alloc_result.as_any_value_enum().into_pointer_value();
        let has_succeeded = ctx.builder.build_is_not_null(new_arr, "has_succeeded");
        ctx.builder.build_conditional_branch(has_succeeded, alloc_success, alloc_failed);



        ctx.builder.position_at_end(alloc_failed);
        let i8_ptr_type = ctx.context.i8_type().ptr_type(AddressSpace::Generic);
        let error_msg = ctx.builder.build_pointer_cast(self.resize_failed.as_pointer_value(), i8_ptr_type, "err_msg");
        ctx.builder.build_call(self.puts, &[error_msg.into()], "");
        free_variables(ctx, self, &Variables {
            array,
            len,
            index,
        });
        exit_program(ctx, self);


        ctx.builder.position_at_end(alloc_success);
        let old_arr = ctx.builder.build_load(array, "old_arr");
        let old_len = len_val;
        let val_0 = ctx.context.i32_type().const_int(0, false);
        ctx.builder.build_call(self.memset, &[new_arr.into(), val_0.into(), new_len.into()], "");
        ctx.builder.build_call(self.memcpy, &[new_arr.into(), old_arr.into(), old_len.into()], "");
        ctx.builder.build_call(self.free, &[old_arr.into()], "");
        ctx.builder.build_store(array, new_arr);
        ctx.builder.build_store(len, new_len);
        ctx.builder.build_unconditional_branch(resized);


        ctx.builder.position_at_end(resized);
        ctx.builder.build_return(None);


        self.resize = Some(resize);
    }
    fn build_next(&mut self, ctx: &CompilationContext<'ctx>) {
        let i64_ptr_t = ctx.context.i64_type().ptr_type(AddressSpace::Generic);
        let i64_t = ctx.context.i64_type();
        let void_t = ctx.context.void_type();

        let fn_type = void_t.fn_type(&[i64_ptr_t.into(), i64_t.into()], false);
        let next = ctx.module.add_function("next", fn_type, None);

        let entry = ctx.context.append_basic_block(next, "entry");
        ctx.builder.position_at_end(entry);

        let index = next.get_nth_param(0).unwrap().into_pointer_value();
        let amount_val = next.get_nth_param(1).unwrap().into_int_value();



        let old_i = ctx.builder.build_load(index, "old_i");
        let new_i = ctx.builder.build_int_add(old_i.into_int_value(), amount_val, "new_i");
        ctx.builder.build_store(index, new_i);
        ctx.builder.build_return(None);

        self.next = Some(next);
    }
    fn build_previous(&mut self, ctx: &CompilationContext<'ctx>) {
        let i64_ptr_t = ctx.context.i64_type().ptr_type(AddressSpace::Generic);
        let i64_t = ctx.context.i64_type();
        let i8_ptr_ptr_t = ctx.context.i8_type().ptr_type(AddressSpace::Generic).ptr_type(AddressSpace::Generic);
        let void_t = ctx.context.void_type();

        let fn_type = void_t.fn_type(&[i8_ptr_ptr_t.into(), i64_ptr_t.into(), i64_ptr_t.into(), i64_t.into()], false);
        let previous = ctx.module.add_function("previous", fn_type, None);

        let entry = ctx.context.append_basic_block(previous, "entry");
        ctx.builder.position_at_end(entry);


        let array = previous.get_nth_param(0).unwrap().into_pointer_value();
        let len = previous.get_nth_param(1).unwrap().into_pointer_value();
        let index = previous.get_nth_param(2).unwrap().into_pointer_value();
        let amount_val = previous.get_nth_param(3).unwrap().into_int_value();


        let underflowed = ctx.context.append_basic_block(previous, "underflowed");
        let not_underflowed = ctx.context.append_basic_block(previous, "not_underflowed");


        let old_i = ctx.builder.build_load(index, "old_i");

        let is_underflow = ctx.builder.build_int_compare(IntPredicate::UGT, amount_val, old_i.into_int_value(), "is_underflow");
        ctx.builder.build_conditional_branch(is_underflow, underflowed, not_underflowed);

        ctx.builder.position_at_end(underflowed);
        let i8_ptr_type = ctx.context.i8_type().ptr_type(AddressSpace::Generic);
        let error_msg = ctx.builder.build_pointer_cast(self.index_underflow.as_pointer_value(), i8_ptr_type, "err_msg");
        ctx.builder.build_call(self.puts, &[error_msg.into()], "");
        free_variables(ctx, self, &Variables {
            array,
            len,
            index,
        });
        exit_program(ctx, self);


        ctx.builder.position_at_end(not_underflowed);
        let new_i = ctx.builder.build_int_sub(old_i.into_int_value(), amount_val, "new_i");
        ctx.builder.build_store(index, new_i);
        ctx.builder.build_return(None);

        self.previous = Some(previous);
    }
    fn build_increment(&mut self, ctx: &CompilationContext<'ctx>) {
        let i64_ptr_t = ctx.context.i64_type().ptr_type(AddressSpace::Generic);
        let i8_ptr_ptr_t = ctx.context.i8_type().ptr_type(AddressSpace::Generic).ptr_type(AddressSpace::Generic);
        let i8_t = ctx.context.i8_type();
        let void_t = ctx.context.void_type();

        let fn_type = void_t.fn_type(&[i8_ptr_ptr_t.into(), i64_ptr_t.into(), i64_ptr_t.into(), i8_t.into()], false);
        let increment = ctx.module.add_function("increment", fn_type, None);

        let entry = ctx.context.append_basic_block(increment, "entry");
        ctx.builder.position_at_end(entry);

        let array = increment.get_nth_param(0).unwrap().into_pointer_value();
        let len = increment.get_nth_param(1).unwrap().into_pointer_value();
        let index = increment.get_nth_param(2).unwrap().into_pointer_value();
        let amount_val = increment.get_nth_param(3).unwrap().into_int_value();


        ctx.builder.build_call(self.resize(), &[array.into(), len.into(), index.into()], "");

        let i_val = ctx.builder.build_load(index, "i_val").into_int_value();
        let arr_ptr = ctx.builder.build_load(array, "arr_ptr").into_pointer_value();
        let cell_ptr = unsafe { ctx.builder.build_gep(arr_ptr, &[i_val], "cell_ptr") };

        let cell_val = ctx.builder.build_load(cell_ptr, "cell_val").into_int_value();
        let new_cell_val = ctx.builder.build_int_add(cell_val, amount_val, "new_cell_val");
        ctx.builder.build_store(cell_ptr, new_cell_val);
        ctx.builder.build_return(None);


        self.increment = Some(increment);
    }
    fn build_decrement(&mut self, ctx: &CompilationContext<'ctx>) {
        let i64_ptr_t = ctx.context.i64_type().ptr_type(AddressSpace::Generic);
        let i8_ptr_ptr_t = ctx.context.i8_type().ptr_type(AddressSpace::Generic).ptr_type(AddressSpace::Generic);
        let i8_t = ctx.context.i8_type();
        let void_t = ctx.context.void_type();

        let fn_type = void_t.fn_type(&[i8_ptr_ptr_t.into(), i64_ptr_t.into(), i64_ptr_t.into(), i8_t.into()], false);
        let decrement = ctx.module.add_function("decrement", fn_type, None);

        let entry = ctx.context.append_basic_block(decrement, "entry");
        ctx.builder.position_at_end(entry);

        let array = decrement.get_nth_param(0).unwrap().into_pointer_value();
        let len = decrement.get_nth_param(1).unwrap().into_pointer_value();
        let index = decrement.get_nth_param(2).unwrap().into_pointer_value();
        let amount_val = decrement.get_nth_param(3).unwrap().into_int_value();


        ctx.builder.build_call(self.resize(), &[array.into(), len.into(), index.into()], "");

        let i_val = ctx.builder.build_load(index, "i_val").into_int_value();
        let arr_ptr = ctx.builder.build_load(array, "arr_ptr").into_pointer_value();
        let cell_ptr = unsafe { ctx.builder.build_gep(arr_ptr, &[i_val], "cell_ptr") };

        let cell_val = ctx.builder.build_load(cell_ptr, "cell_val").into_int_value();
        let new_cell_val = ctx.builder.build_int_sub(cell_val, amount_val, "new_cell_val");
        ctx.builder.build_store(cell_ptr, new_cell_val);
        ctx.builder.build_return(None);

        self.decrement = Some(decrement);
    }
    fn build_output(&mut self, ctx: &CompilationContext<'ctx>) {
        let i64_ptr_t = ctx.context.i64_type().ptr_type(AddressSpace::Generic);
        let i8_ptr_ptr_t = ctx.context.i8_type().ptr_type(AddressSpace::Generic).ptr_type(AddressSpace::Generic);
        let void_t = ctx.context.void_type();

        let fn_type = void_t.fn_type(&[i8_ptr_ptr_t.into(), i64_ptr_t.into(), i64_ptr_t.into()], false);
        let output = ctx.module.add_function("output", fn_type, None);

        let entry = ctx.context.append_basic_block(output, "entry");
        ctx.builder.position_at_end(entry);

        let array = output.get_nth_param(0).unwrap().into_pointer_value();
        let len = output.get_nth_param(1).unwrap().into_pointer_value();
        let index = output.get_nth_param(2).unwrap().into_pointer_value();


        ctx.builder.build_call(self.resize(), &[array.into(), len.into(), index.into()], "");


        let i_val = ctx.builder.build_load(index, "i_val").into_int_value();
        let arr_ptr = ctx.builder.build_load(array, "arr_ptr").into_pointer_value();
        let cell_ptr = unsafe { ctx.builder.build_gep(arr_ptr, &[i_val], "cell_ptr") };

        let cell_val = ctx.builder.build_load(cell_ptr, "cell_val");
        let i32_type = ctx.context.i32_type();
        let out_c = ctx.builder.build_int_cast(cell_val.into_int_value(), i32_type, "out_c");
        ctx.builder.build_call(self.putchar, &[out_c.into()], "");
        ctx.builder.build_call(self.flush_stdout, &[], "");
        ctx.builder.build_return(None);


        self.output = Some(output);
    }
    fn build_input(&mut self, ctx: &CompilationContext<'ctx>) {
        let i64_ptr_t = ctx.context.i64_type().ptr_type(AddressSpace::Generic);
        let i8_ptr_ptr_t = ctx.context.i8_type().ptr_type(AddressSpace::Generic).ptr_type(AddressSpace::Generic);
        let void_t = ctx.context.void_type();

        let fn_type = void_t.fn_type(&[i8_ptr_ptr_t.into(), i64_ptr_t.into(), i64_ptr_t.into()], false);
        let input = ctx.module.add_function("input", fn_type, None);

        let entry = ctx.context.append_basic_block(input, "entry");
        ctx.builder.position_at_end(entry);

        let array = input.get_nth_param(0).unwrap().into_pointer_value();
        let len = input.get_nth_param(1).unwrap().into_pointer_value();
        let index = input.get_nth_param(2).unwrap().into_pointer_value();


        ctx.builder.build_call(self.resize(), &[array.into(), len.into(), index.into()], "");


        let not_eof = ctx.context.append_basic_block(input, "not_eof");
        let input_complete = ctx.context.append_basic_block(input, "input_complete");

        let in_c = ctx.builder.build_call(self.getchar, &[], "in_c");
        let in_c = in_c.as_any_value_enum().into_int_value();
        let val_0 = ctx.context.i32_type().const_int(0, false);
        let is_eof = ctx.builder.build_int_compare(IntPredicate::SLT, in_c, val_0, "is_eof");
        ctx.builder.build_conditional_branch(is_eof, input_complete, not_eof);

        ctx.builder.position_at_end(not_eof);
        let new_cell_val = ctx.builder.build_int_cast(in_c, ctx.context.i8_type(), "new_cell_value");
        let arr_ptr = ctx.builder.build_load(array, "arr_ptr").into_pointer_value();
        let i_val = ctx.builder.build_load(index, "index_val").into_int_value();
        let cell_ptr = unsafe { ctx.builder.build_gep(arr_ptr, &[i_val], "cell_ptr")};
        ctx.builder.build_store(cell_ptr, new_cell_val);
        ctx.builder.build_unconditional_branch(input_complete);

        ctx.builder.position_at_end(input_complete);
        ctx.builder.build_return(None);

        self.input = Some(input);
    }
    fn build_set(&mut self, ctx: &CompilationContext<'ctx>) {
        let i64_ptr_t = ctx.context.i64_type().ptr_type(AddressSpace::Generic);
        let i8_ptr_ptr_t = ctx.context.i8_type().ptr_type(AddressSpace::Generic).ptr_type(AddressSpace::Generic);
        let i8_t = ctx.context.i8_type();
        let void_t = ctx.context.void_type();

        let fn_type = void_t.fn_type(&[i8_ptr_ptr_t.into(), i64_ptr_t.into(), i64_ptr_t.into(), i8_t.into()], false);
        let set = ctx.module.add_function("set", fn_type, None);

        let entry = ctx.context.append_basic_block(set, "entry");
        ctx.builder.position_at_end(entry);

        let array = set.get_nth_param(0).unwrap().into_pointer_value();
        let len = set.get_nth_param(1).unwrap().into_pointer_value();
        let index = set.get_nth_param(2).unwrap().into_pointer_value();
        let value_val = set.get_nth_param(3).unwrap().into_int_value();


        ctx.builder.build_call(self.resize(), &[array.into(), len.into(), index.into()], "");


        let arr_ptr = ctx.builder.build_load(array, "arr_ptr").into_pointer_value();
        let i_val = ctx.builder.build_load(index, "index_val").into_int_value();
        let cell_ptr = unsafe { ctx.builder.build_gep(arr_ptr, &[i_val], "cell_ptr")};
        ctx.builder.build_store(cell_ptr, value_val);
        ctx.builder.build_return(None);


        self.set = Some(set);
    }

    fn build_const_str(ctx: &CompilationContext<'ctx>, val: &str, name: &str) -> GlobalValue<'ctx> {
        let string = Self::str_to_bytes(val, ctx);

        let i8_t = ctx.context.i8_type();
        let t = i8_t.array_type(string.len() as u32);
        let global = ctx.module.add_global(t, None, name);
        let init = i8_t.const_array(&string);

        global.set_initializer(&init);


        global
    }
    fn str_to_bytes(val: &str, ctx: &CompilationContext<'ctx>) -> Vec<IntValue<'ctx>> {
        let mut rets = Vec::new();

        for c in val.chars() {
            rets.push(ctx.context.i8_type().const_int(c as u64, false));
        }
        rets.push(ctx.context.i8_type().const_int(0, false));

        rets
    }
}
struct Variables<'ctx> {
    array: PointerValue<'ctx>,
    len: PointerValue<'ctx>,
    index: PointerValue<'ctx>,
}
impl<'ctx> Variables<'ctx> {
    pub fn new(ctx: &CompilationContext<'ctx>, _symbols: &Symbols) -> Variables<'ctx> {
        let array = ctx.builder.build_alloca(ctx.context.i8_type().ptr_type(AddressSpace::Generic), "arr");
        let len = ctx.builder.build_alloca(ctx.context.i64_type(), "len");
        let index = ctx.builder.build_alloca(ctx.context.i64_type(), "index");

        Variables {
            array,
            len,
            index,
        }
    }
}
