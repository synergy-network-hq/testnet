use quantumvm::{Assembler, OpCode, QuantumVM};

#[test]
fn test_basic_arithmetic() {
    let mut assembler = Assembler::new();
    assembler.emit_op(OpCode::Push);
    assembler.emit_i32(10);
    assembler.emit_op(OpCode::Push);
    assembler.emit_i32(20);
    assembler.emit_op(OpCode::Add);
    assembler.emit_op(OpCode::Halt);

    let bytecode = assembler.build();
    let mut vm = QuantumVM::new();
    vm.load_bytecode(&bytecode).unwrap();
    vm.execute().unwrap();

    let result = vm.stack.pop().unwrap().as_i32().unwrap();
    assert_eq!(result, 30);
}

#[test]
fn test_dilithium_verify_shim() {
    let mut assembler = Assembler::new();

    // The arguments are popped in reverse order of how they are pushed.
    // So we push pk, then message, then signature.
    // The verify function expects (message, signature, pk).

    // Push mock public key
    assembler.emit_op(OpCode::LoadImm);
    assembler.emit_bytes(&vec![0x01; pqc_shims::dilithium::DILITHIUM_PUBLIC_KEY_BYTES]);

    // Push mock message
    assembler.emit_op(OpCode::LoadImm);
    assembler.emit_bytes(b"Hello, quantum world!");

    // Push mock signature
    assembler.emit_op(OpCode::LoadImm);
    assembler.emit_bytes(&vec![0x02; pqc_shims::dilithium::DILITHIUM_SIGNATURE_BYTES]);

    // Verify signature
    assembler.emit_op(OpCode::DilithiumVerify);
    assembler.emit_op(OpCode::Halt);

    let bytecode = assembler.build();
    let mut vm = QuantumVM::new();
    vm.load_bytecode(&bytecode).unwrap();
    vm.execute().unwrap();

    let result = vm.stack.pop().unwrap().as_bool().unwrap();
    assert_eq!(result, true);
}

#[test]
fn test_kyber_decaps_shim() {
    let mut assembler = Assembler::new();

    // The decaps function expects (ciphertext, private_key)
    // So we push private_key, then ciphertext.

    // Push mock private key
    assembler.emit_op(OpCode::LoadImm);
    assembler.emit_bytes(&vec![0x01; pqc_shims::kyber::KYBER_SECRET_KEY_BYTES]);

    // Push mock ciphertext
    assembler.emit_op(OpCode::LoadImm);
    assembler.emit_bytes(&vec![0x02; pqc_shims::kyber::KYBER_CIPHERTEXT_BYTES]);

    // Decapsulate
    assembler.emit_op(OpCode::KyberKeyExchange); // This opcode maps to decaps
    assembler.emit_op(OpCode::Halt);

    let bytecode = assembler.build();
    let mut vm = QuantumVM::new();
    vm.load_bytecode(&bytecode).unwrap();
    vm.execute().unwrap();

    let shared_secret = vm.stack.pop().unwrap().as_bytes().unwrap().to_vec();
    assert_eq!(shared_secret, vec![0u8; pqc_shims::kyber::KYBER_SHARED_SECRET_BYTES]);
}
