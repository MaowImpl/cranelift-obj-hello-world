#![allow(dead_code, unused)]

use std::fs::File;
use anyhow::Result;
use cranelift::prelude::*;
use cranelift::prelude::types::*;
use cranelift::codegen::ir::Function;
use cranelift::codegen::isa::{lookup, CallConv};
use cranelift::codegen::settings::{self, Configurable};
use cranelift_module::{Module, DataContext, Linkage, default_libcall_names};
use cranelift_object::{ObjectBuilder, ObjectModule};
use target_lexicon::Triple;
use crate::settings::Flags;

fn make_module(triple: Triple, flags: Flags, name: &str) -> Result<ObjectModule> {
    let builder = ObjectBuilder::new(
        lookup(triple)?.finish(flags)?,
        name,
        default_libcall_names()
    )?;
    Ok(ObjectModule::new(builder))
}

fn make_signature(module: &dyn Module, return_type: Type, argument_types: &[Type]) -> Signature {
    let mut signature = module.make_signature();
    signature.returns.push(AbiParam::new(return_type));
    for t in argument_types {
        signature.params.push(AbiParam::new(*t));
    }
    signature
}

fn main() -> Result<()> {
    // Set flags
    let mut flags = settings::builder();
    flags.set("opt_level", "speed")?;

    // Get module information
    let mut module = make_module(
        Triple::host(),
        Flags::new(flags),
        "test_module"
    )?;
    let pointer_type = module.target_config().pointer_type();

    // Acquire contexts
    let mut function_ctx = FunctionBuilderContext::new();
    let mut data_ctx = DataContext::new();
    let mut ctx = module.make_context();

    // Declare 'puts' from the C standard library and gain a reference to it
    let puts_id = module.declare_function(
        "puts",
        Linkage::Import,
        &make_signature(&module, I32, &[pointer_type])
    )?;
    let puts_reference = module.declare_func_in_func(puts_id, &mut ctx.func);

    // Declare a global accessible from this module named "hello_world"
    let data_id = module.declare_data(
        "hello_world",
        Linkage::Local,
        true,
        false
    )?;

    // Define the contents of "hello_world" as such: "Hello, World!"
    data_ctx.define(b"Hello, World!\0".to_vec().into_boxed_slice());
    module.define_data(data_id, &data_ctx)?;
    data_ctx.clear();

    // Acquire a reference to the global
    let data_id = module.declare_data_in_func(data_id, &mut ctx.func);

    // Build function IR
    {
        let mut builder = FunctionBuilder::new(&mut ctx.func, &mut function_ctx);

        // Create block
        let block = builder.create_block();

        builder.append_block_params_for_function_params(block);
        builder.switch_to_block(block);
        builder.seal_block(block);

        // Acquire a pointer to the "hello_world" global, call 'puts' with it, and then return.
        let value = builder.ins().symbol_value(pointer_type, data_id);
        builder.ins().call(puts_reference, &[value]);
        builder.ins().return_(&[]);

        // Finalize function.
        builder.finalize();
    }

    // Declare and define our entrypoint function
    let function_id = module.declare_function(
        "main",
        Linkage::Export,
        &ctx.func.signature
    )?;
    module.define_function(function_id, &mut ctx)?;

    module.clear_context(&mut ctx);

    // Output compiled module to object file
    let object = module.finish().object;
    let file = File::create("test_module.o")?;
    object
        .write_stream(file)
        .expect("Could not write to object file");

    Ok(())
}
