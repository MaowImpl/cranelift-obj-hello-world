# cranelift-obj-hello-world
Cranelift test that generates an object file for a "Hello, World!" program

1. Compile and run `cranelift-obj-hello-world`
2. Link the generated object file, i.e. `clang -o test_module.exe test_module.o`
3. Run the executable
4. Hello, World!

This test/example of Cranelift is decently commented.
I mainly created it to understand the library's API, but I hope it will help future Cranelift users out.
