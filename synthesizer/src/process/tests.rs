// Copyright (C) 2019-2023 Aleo Systems Inc.
// This file is part of the snarkVM library.

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at:
// http://www.apache.org/licenses/LICENSE-2.0

// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use super::*;
use crate::store::{
    helpers::memory::{BlockMemory, FinalizeMemory},
    BlockStore,
};
use circuit::{network::AleoV0, Assignment};
use console::{
    account::{Address, PrivateKey, ViewKey},
    network::Testnet3,
    program::{Identifier, Literal, Value},
    types::Field,
};
use snarkvm_curves::bls12_377::Fr;

type CurrentNetwork = Testnet3;
type CurrentAleo = AleoV0;

#[test]
fn test_process_execute_mint() {
    // Initialize a new program.
    let program = Program::<CurrentNetwork>::credits().unwrap();

    // Initialize the RNG.
    let rng = &mut TestRng::default();
    // Initialize a new caller account.
    let caller_private_key = PrivateKey::<CurrentNetwork>::new(rng).unwrap();
    let _caller_view_key = ViewKey::try_from(&caller_private_key).unwrap();
    let caller = Address::try_from(&caller_private_key).unwrap();
    // Declare the input value.
    let r0 = Value::<CurrentNetwork>::from_str(&format!("{caller}")).unwrap();
    let r1 = Value::<CurrentNetwork>::from_str("99_000_000_000_000_u64").unwrap();

    // Construct the process.
    let process = Process::load().unwrap();

    // Authorize the function call.
    let authorization = process
        .authorize::<CurrentAleo, _>(
            &caller_private_key,
            program.id(),
            Identifier::from_str("mint").unwrap(),
            [r0, r1].iter(),
            rng,
        )
        .unwrap();
    assert_eq!(authorization.len(), 1);
    let request = authorization.peek_next().unwrap();

    // Compute the encryption randomizer as `HashToScalar(tvk || index)`.
    let randomizer = CurrentNetwork::hash_to_scalar_psd2(&[*request.tvk(), Field::from_u64(3)]).unwrap();
    let nonce = CurrentNetwork::g_scalar_multiply(&randomizer);

    // Declare the expected output value.
    let r3 = Value::from_str(&format!(
        "{{ owner: {caller}.private, microcredits: 99_000_000_000_000_u64.private, _nonce: {nonce}.public }}"
    ))
    .unwrap();

    // Check again to make sure we didn't modify the authorization before calling `evaluate`.
    assert_eq!(authorization.len(), 1);

    // Compute the output value.
    let response = process.evaluate::<CurrentAleo>(authorization.replicate()).unwrap();
    let candidate = response.outputs();
    assert_eq!(1, candidate.len());
    assert_eq!(r3, candidate[0]);

    // Check again to make sure we didn't modify the authorization after calling `evaluate`.
    assert_eq!(authorization.len(), 1);

    // Execute the request.
    let (response, _trace) = process.execute::<CurrentAleo>(authorization).unwrap();
    let candidate = response.outputs();
    assert_eq!(1, candidate.len());
    assert_eq!(r3, candidate[0]);

    // process.verify_execution::<true>(&execution).unwrap();

    // use circuit::Environment;
    //
    // assert_eq!(22152, CurrentAleo::num_constants());
    // assert_eq!(9, CurrentAleo::num_public());
    // assert_eq!(20561, CurrentAleo::num_private());
    // assert_eq!(20579, CurrentAleo::num_constraints());
    // assert_eq!(79386, CurrentAleo::num_gates());
}

#[test]
fn test_process_circuit_key() {
    // Initialize a new program.
    let program = Program::<CurrentNetwork>::from_str(
        r#"program testing.aleo;

function hello_world:
    input r0 as u32.public;
    input r1 as u32.private;
    add r0 r1 into r2;
    output r2 as u32.private;
"#,
    )
    .unwrap();

    // Declare the function name.
    let function_name = Identifier::from_str("hello_world").unwrap();

    // Construct the process.
    let process = super::test_helpers::sample_process(&program);
    // Check that the circuit key can be synthesized.
    process.synthesize_key::<CurrentAleo, _>(program.id(), &function_name, &mut TestRng::default()).unwrap();
}

#[test]
fn test_process_multirecords() {
    // Initialize a new program.
    let program = Program::<CurrentNetwork>::from_str(
        r"program multirecord.aleo;

  record record_a:
    owner as address.private;
    item as u64.private;

  record record_b:
    owner as address.private;
    item as u64.private;

  record record_c:
    owner as address.private;
    item as u64.private;

  function initialize:
    input r0 as record_a.record;
    input r1 as record_b.record;
    input r2 as record_c.record;
    cast r0.owner r0.item into r3 as record_a.record;
    cast r1.owner r1.item into r4 as record_b.record;
    cast r2.owner r2.item into r5 as record_c.record;
    output r3 as record_a.record;
    output r4 as record_b.record;
    output r5 as record_c.record;",
    )
    .unwrap();

    // Declare the function name.
    let function_name = Identifier::from_str("initialize").unwrap();

    // Initialize the RNG.
    let rng = &mut TestRng::default();

    // Construct the process.
    let process = super::test_helpers::sample_process(&program);

    // Initialize a new caller account.
    let caller_private_key = PrivateKey::<CurrentNetwork>::new(rng).unwrap();
    let _caller_view_key = ViewKey::try_from(&caller_private_key).unwrap();
    let caller = Address::try_from(&caller_private_key).unwrap();

    // Declare the input value.
    let input_a =
        Value::from_str(&format!("{{ owner: {caller}.private, item: 1234u64.private, _nonce: 0group.public }}"))
            .unwrap();
    let input_b =
        Value::from_str(&format!("{{ owner: {caller}.private, item: 4321u64.private, _nonce: 0group.public }}"))
            .unwrap();
    let input_c =
        Value::from_str(&format!("{{ owner: {caller}.private, item: 5678u64.private, _nonce: 0group.public }}"))
            .unwrap();

    // Authorize the function call.
    let authorization = process
        .authorize::<CurrentAleo, _>(
            &caller_private_key,
            program.id(),
            function_name,
            [input_a, input_b, input_c].iter(),
            rng,
        )
        .unwrap();
    assert_eq!(authorization.len(), 1);
    let request = authorization.peek_next().unwrap();

    // Compute the encryption randomizer for the first output as `HashToScalar(tvk || index)`.
    let randomizer_a = CurrentNetwork::hash_to_scalar_psd2(&[*request.tvk(), Field::from_u64(3)]).unwrap();
    let nonce_a = CurrentNetwork::g_scalar_multiply(&randomizer_a);

    // Compute the encryption randomizer for the second output as `HashToScalar(tvk || index)`.
    let randomizer_b = CurrentNetwork::hash_to_scalar_psd2(&[*request.tvk(), Field::from_u64(4)]).unwrap();
    let nonce_b = CurrentNetwork::g_scalar_multiply(&randomizer_b);

    // Compute the encryption randomizer for the third output as `HashToScalar(tvk || index)`.
    let randomizer_c = CurrentNetwork::hash_to_scalar_psd2(&[*request.tvk(), Field::from_u64(5)]).unwrap();
    let nonce_c = CurrentNetwork::g_scalar_multiply(&randomizer_c);

    // Declare the output value.
    let output_a =
        Value::from_str(&format!("{{ owner: {caller}.private, item: 1234u64.private, _nonce: {nonce_a}.public }}"))
            .unwrap();
    let output_b =
        Value::from_str(&format!("{{ owner: {caller}.private, item: 4321u64.private, _nonce: {nonce_b}.public }}"))
            .unwrap();
    let output_c =
        Value::from_str(&format!("{{ owner: {caller}.private, item: 5678u64.private, _nonce: {nonce_c}.public }}"))
            .unwrap();

    // Check again to make sure we didn't modify the authorization before calling `evaluate`.
    assert_eq!(authorization.len(), 1);

    // Compute the output value.
    let response = process.evaluate::<CurrentAleo>(authorization.replicate()).unwrap();
    let candidate = response.outputs();
    assert_eq!(3, candidate.len());
    assert_eq!(output_a, candidate[0]);
    assert_eq!(output_b, candidate[1]);
    assert_eq!(output_c, candidate[2]);

    // Check again to make sure we didn't modify the authorization after calling `evaluate`.
    assert_eq!(authorization.len(), 1);

    // Execute the request.
    let (response, _trace) = process.execute::<CurrentAleo>(authorization).unwrap();
    let candidate = response.outputs();
    assert_eq!(3, candidate.len());
    assert_eq!(output_a, candidate[0]);
    assert_eq!(output_b, candidate[1]);
    assert_eq!(output_c, candidate[2]);

    // use circuit::Environment;
    //
    // assert_eq!(20060, CurrentAleo::num_constants());
    // assert_eq!(12, CurrentAleo::num_public());
    // assert_eq!(57602, CurrentAleo::num_private());
    // assert_eq!(57684, CurrentAleo::num_constraints());
    // assert_eq!(178189, CurrentAleo::num_gates());
}

#[test]
fn test_process_self_caller() {
    // Initialize a new program.
    let program = Program::<CurrentNetwork>::from_str(
        r"program caller.aleo;

  record data:
    owner as address.private;
    item as u64.private;

  function initialize:
    input r0 as data.record;
    cast self.caller r0.item into r1 as data.record;
    output r1 as data.record;",
    )
    .unwrap();

    // Declare the function name.
    let function_name = Identifier::from_str("initialize").unwrap();

    // Initialize the RNG.
    let rng = &mut TestRng::default();

    // Construct the process.
    let process = super::test_helpers::sample_process(&program);

    // Initialize a new caller account.
    let caller_private_key = PrivateKey::<CurrentNetwork>::new(rng).unwrap();
    let _caller_view_key = ViewKey::try_from(&caller_private_key).unwrap();
    let caller = Address::try_from(&caller_private_key).unwrap();

    // Declare the input value.
    let input =
        Value::from_str(&format!("{{ owner: {caller}.private, item: 1234u64.private, _nonce: 0group.public }}"))
            .unwrap();

    // Authorize the function call.
    let authorization = process
        .authorize::<CurrentAleo, _>(&caller_private_key, program.id(), function_name, [input].iter(), rng)
        .unwrap();
    assert_eq!(authorization.len(), 1);
    let request = authorization.peek_next().unwrap();

    // Compute the encryption randomizer as `HashToScalar(tvk || index)`.
    let randomizer = CurrentNetwork::hash_to_scalar_psd2(&[*request.tvk(), Field::from_u64(1)]).unwrap();
    let nonce = CurrentNetwork::g_scalar_multiply(&randomizer);

    // Declare the output value.
    let output =
        Value::from_str(&format!("{{ owner: {caller}.private, item: 1234u64.private, _nonce: {nonce}.public }}"))
            .unwrap();

    // Check again to make sure we didn't modify the authorization before calling `evaluate`.
    assert_eq!(authorization.len(), 1);

    // Compute the output value.
    let response = process.evaluate::<CurrentAleo>(authorization.replicate()).unwrap();
    let candidate = response.outputs();
    assert_eq!(1, candidate.len());
    assert_eq!(output, candidate[0]);

    // Check again to make sure we didn't modify the authorization after calling `evaluate`.
    assert_eq!(authorization.len(), 1);

    // Execute the request.
    let (response, _trace) = process.execute::<CurrentAleo>(authorization).unwrap();
    let candidate = response.outputs();
    assert_eq!(1, candidate.len());
    assert_eq!(output, candidate[0]);
}

#[test]
fn test_process_program_id() {
    // Initialize a new program.
    let program = Program::<CurrentNetwork>::from_str(
        r"program id.aleo;

  struct data:
    user as address;

  function initialize:
    cast id.aleo into r0 as data;
    output r0 as data.private;",
    )
    .unwrap();

    // Declare the program ID.
    let program_id = ProgramID::from_str("id.aleo").unwrap();
    assert_eq!(*program.id(), program_id);

    // Declare the function name.
    let function_name = Identifier::from_str("initialize").unwrap();

    // Initialize the RNG.
    let rng = &mut TestRng::default();

    // Construct the process.
    let process = super::test_helpers::sample_process(&program);

    // Initialize a new caller account.
    let caller_private_key = PrivateKey::<CurrentNetwork>::new(rng).unwrap();

    // Authorize the function call.
    let inputs: &[Value<CurrentNetwork>] = &[];
    let authorization = process
        .authorize::<CurrentAleo, _>(&caller_private_key, program.id(), function_name, inputs.iter(), rng)
        .unwrap();
    assert_eq!(authorization.len(), 1);

    // Declare the output value.
    let output = Value::from_str(&format!("{{ user: {} }}", program_id.to_address().unwrap())).unwrap();

    // Check again to make sure we didn't modify the authorization before calling `evaluate`.
    assert_eq!(authorization.len(), 1);

    // Compute the output value.
    let response = process.evaluate::<CurrentAleo>(authorization.replicate()).unwrap();
    let candidate = response.outputs();
    assert_eq!(1, candidate.len());
    assert_eq!(output, candidate[0]);

    // Check again to make sure we didn't modify the authorization after calling `evaluate`.
    assert_eq!(authorization.len(), 1);

    // Execute the request.
    let (response, _trace) = process.execute::<CurrentAleo>(authorization).unwrap();
    let candidate = response.outputs();
    assert_eq!(1, candidate.len());
    assert_eq!(output, candidate[0]);
}

#[test]
fn test_process_output_operand() {
    // Helper function to test authorization, execution, and verification for the program below.
    fn authorize_execute_and_verify(
        program: &Program<CurrentNetwork>,
        function_name: Identifier<CurrentNetwork>,
        output: Value<CurrentNetwork>,
        caller_private_key: &PrivateKey<CurrentNetwork>,
        rng: &mut TestRng,
    ) {
        // Construct the process.
        let process = super::test_helpers::sample_process(program);

        // Authorize the function call.
        let inputs: &[Value<CurrentNetwork>] = &[];
        let authorization = process
            .authorize::<CurrentAleo, _>(caller_private_key, program.id(), function_name, inputs.iter(), rng)
            .unwrap();
        assert_eq!(authorization.len(), 1);

        // Check again to make sure we didn't modify the authorization before calling `evaluate`.
        assert_eq!(authorization.len(), 1);

        // Compute the output value.
        let response = process.evaluate::<CurrentAleo>(authorization.replicate()).unwrap();
        let candidate = response.outputs();
        assert_eq!(1, candidate.len());
        assert_eq!(output, candidate[0]);

        // Check again to make sure we didn't modify the authorization after calling `evaluate`.
        assert_eq!(authorization.len(), 1);

        // Execute the request.
        let (response, _trace) = process.execute::<CurrentAleo>(authorization).unwrap();
        let candidate = response.outputs();
        assert_eq!(1, candidate.len());
        assert_eq!(output, candidate[0]);

        // process.verify_execution::<true>(&execution).unwrap();
    }

    // Initialize a new program.
    let program = Program::<CurrentNetwork>::from_str(
        r"program operand.aleo;

  function program_id:
    output operand.aleo as address.private;

  function literal:
    output 1234u64 as u64.private;

  function caller:
    output self.caller as address.private;",
    )
    .unwrap();

    // Initalize the RNG.
    let rng = &mut TestRng::default();

    // Initialize a new caller account.
    let caller_private_key = PrivateKey::<CurrentNetwork>::new(rng).unwrap();
    let caller = Address::try_from(&caller_private_key).unwrap();

    // Test the `program_id` function.
    authorize_execute_and_verify(
        &program,
        Identifier::from_str("program_id").unwrap(),
        Value::from_str(&program.id().to_address().unwrap().to_string()).unwrap(),
        &caller_private_key,
        rng,
    );

    // Test the `literal` function.
    authorize_execute_and_verify(
        &program,
        Identifier::from_str("literal").unwrap(),
        Value::from_str("1234u64").unwrap(),
        &caller_private_key,
        rng,
    );

    // Test the `caller` function.
    authorize_execute_and_verify(
        &program,
        Identifier::from_str("caller").unwrap(),
        Value::from_str(&format!("{caller}")).unwrap(),
        &caller_private_key,
        rng,
    );
}

#[test]
fn test_process_execute_call_closure() {
    // Initialize a new program.
    let (string, program) = Program::<CurrentNetwork>::parse(
        r"
program token.aleo;

record token:
    owner as address.private;
    token_amount as u64.private;

// (a + (a + b)) + (a + b) == (3a + 2b)
closure execute:
    input r0 as field;
    input r1 as field;
    add r0 r1 into r2;
    add r0 r2 into r3;
    add r2 r3 into r4;
    output r4 as field;
    output r3 as field;
    output r2 as field;

closure check_not_equal:
    input r0 as field;
    input r1 as field;
    assert.neq r0 r1;

function compute:
    input r0 as field.private;
    input r1 as field.public;
    input r2 as token.record;
    cast r2.owner r2.token_amount into r3 as token.record;
    call check_not_equal r0 r1;
    call execute r0 r1 into r4 r5 r6;
    output r3 as token.record;
    output r4 as field.private;
    output r5 as field.private;
    output r6 as field.private;",
    )
    .unwrap();
    assert!(string.is_empty(), "Parser did not consume all of the string: '{string}'");

    // Declare the function name.
    let function_name = Identifier::from_str("compute").unwrap();

    // Initialize the RNG.
    let rng = &mut TestRng::default();

    // Construct the process.
    let process = super::test_helpers::sample_process(&program);
    // Check that the circuit key can be synthesized.
    process.synthesize_key::<CurrentAleo, _>(program.id(), &function_name, rng).unwrap();

    // Reset the process.
    let process = super::test_helpers::sample_process(&program);

    // Initialize a new caller account.
    let caller_private_key = PrivateKey::<CurrentNetwork>::new(rng).unwrap();
    let _caller_view_key = ViewKey::try_from(&caller_private_key).unwrap();
    let caller = Address::try_from(&caller_private_key).unwrap();

    // Prepare a record belonging to the address.
    let record_string = format!("{{ owner: {caller}.private, token_amount: 100u64.private, _nonce: 0group.public }}");

    // Declare the input value.
    let r0 = Value::<CurrentNetwork>::from_str("3field").unwrap();
    let r1 = Value::<CurrentNetwork>::from_str("5field").unwrap();
    let r2 = Value::<CurrentNetwork>::from_str(&record_string).unwrap();

    // Authorize the function call.
    let authorization = process
        .authorize::<CurrentAleo, _>(&caller_private_key, program.id(), function_name, [r0, r1, r2].iter(), rng)
        .unwrap();
    assert_eq!(authorization.len(), 1);
    let request = authorization.peek_next().unwrap();

    // Compute the encryption randomizer as `HashToScalar(tvk || index)`.
    let randomizer = CurrentNetwork::hash_to_scalar_psd2(&[*request.tvk(), Field::from_u64(3)]).unwrap();
    let nonce = CurrentNetwork::g_scalar_multiply(&randomizer);

    // Declare the expected output value.
    let r3 = Value::<CurrentNetwork>::from_str(&format!(
        "{{ owner: {caller}.private, token_amount: 100u64.private, _nonce: {nonce}.public }}"
    ))
    .unwrap();
    let r4 = Value::from_str("19field").unwrap();
    let r5 = Value::from_str("11field").unwrap();
    let r6 = Value::from_str("8field").unwrap();

    // Check again to make sure we didn't modify the authorization before calling `evaluate`.
    assert_eq!(authorization.len(), 1);

    // Compute the output value.
    let response = process.evaluate::<CurrentAleo>(authorization.replicate()).unwrap();
    let candidate = response.outputs();
    assert_eq!(4, candidate.len());
    assert_eq!(r3, candidate[0]);
    assert_eq!(r4, candidate[1]);
    assert_eq!(r5, candidate[2]);
    assert_eq!(r6, candidate[3]);

    // Check again to make sure we didn't modify the authorization after calling `evaluate`.
    assert_eq!(authorization.len(), 1);

    // Execute the request.
    let (response, _trace) = process.execute::<CurrentAleo>(authorization).unwrap();
    let candidate = response.outputs();
    assert_eq!(4, candidate.len());
    assert_eq!(r3, candidate[0]);
    assert_eq!(r4, candidate[1]);
    assert_eq!(r5, candidate[2]);
    assert_eq!(r6, candidate[3]);

    // use circuit::Environment;
    //
    // assert_eq!(37080, CurrentAleo::num_constants());
    // assert_eq!(12, CurrentAleo::num_public());
    // assert_eq!(41630, CurrentAleo::num_private());
    // assert_eq!(41685, CurrentAleo::num_constraints());
    // assert_eq!(159387, CurrentAleo::num_gates());
}

#[test]
fn test_process_execute_call_external_function() {
    // Initialize a new program.
    let (string, program0) = Program::<CurrentNetwork>::parse(
        r"
program token.aleo;

record token:
    owner as address.private;
    amount as u64.private;

function mint:
    input r0 as address.private;
    input r1 as u64.private;
    cast r0 r1 into r2 as token.record;
    output r2 as token.record;

function produce_magic_number:
    add 1234u64 0u64 into r0;
    output r0 as u64.private;

function check_magic_number:
    input r0 as u64.private;
    assert.eq r0 1234u64;

function noop:

function transfer:
    input r0 as token.record;
    input r1 as address.private;
    input r2 as u64.private;
    sub r0.amount r2 into r3;
    cast r1 r2 into r4 as token.record;
    cast r0.owner r3 into r5 as token.record;
    output r4 as token.record;
    output r5 as token.record;",
    )
    .unwrap();
    assert!(string.is_empty(), "Parser did not consume all of the string: '{string}'");

    // Construct the process.
    let mut process = super::test_helpers::sample_process(&program0);
    // Initialize another program.
    let (string, program1) = Program::<CurrentNetwork>::parse(
        r"
import token.aleo;

program wallet.aleo;

function transfer:
    input r0 as token.aleo/token.record;
    input r1 as address.private;
    input r2 as u64.private;
    call token.aleo/noop;
    call token.aleo/produce_magic_number into r3;
    call token.aleo/check_magic_number r3;
    call token.aleo/transfer r0 r1 r2 into r4 r5;
    output r4 as token.aleo/token.record;
    output r5 as token.aleo/token.record;",
    )
    .unwrap();
    assert!(string.is_empty(), "Parser did not consume all of the string: '{string}'");

    // Add the program to the process.
    process.add_program(&program1).unwrap();

    // Initialize the RNG.
    let rng = &mut TestRng::default();

    // Initialize caller 0.
    let caller0_private_key = PrivateKey::<CurrentNetwork>::new(rng).unwrap();
    let caller0 = Address::try_from(&caller0_private_key).unwrap();

    // Initialize caller 1.
    let caller1_private_key = PrivateKey::<CurrentNetwork>::new(rng).unwrap();
    let caller1 = Address::try_from(&caller1_private_key).unwrap();

    // Declare the function name.
    let function_name = Identifier::from_str("transfer").unwrap();

    // Declare the input value.
    let r0 = Value::<CurrentNetwork>::from_str(&format!(
        "{{ owner: {caller0}.private, amount: 100u64.private, _nonce: 0group.public }}"
    ))
    .unwrap();
    let r1 = Value::<CurrentNetwork>::from_str(&caller1.to_string()).unwrap();
    let r2 = Value::<CurrentNetwork>::from_str("99u64").unwrap();

    // Authorize the function call.
    let authorization = process
        .authorize::<CurrentAleo, _>(&caller0_private_key, program1.id(), function_name, [r0, r1, r2].iter(), rng)
        .unwrap();
    assert_eq!(authorization.len(), 5);
    println!("\nAuthorize\n{:#?}\n\n", authorization.to_vec_deque());

    let (output_a, output_b) = {
        // Fetch the first request.
        let request = authorization.to_vec_deque().pop_back().unwrap();

        // Compute the encryption randomizer as `HashToScalar(tvk || index)`.
        let randomizer_a = CurrentNetwork::hash_to_scalar_psd2(&[*request.tvk(), Field::from_u64(4)]).unwrap();
        let nonce_a = CurrentNetwork::g_scalar_multiply(&randomizer_a);

        // Compute the encryption randomizer as `HashToScalar(tvk || index)`.
        let randomizer_b = CurrentNetwork::hash_to_scalar_psd2(&[*request.tvk(), Field::from_u64(5)]).unwrap();
        let nonce_b = CurrentNetwork::g_scalar_multiply(&randomizer_b);

        // Declare the expected output value.
        let output_a = Value::from_str(&format!(
            "{{ owner: {caller1}.private, amount: 99u64.private, _nonce: {nonce_a}.public }}"
        ))
        .unwrap();
        let output_b =
            Value::from_str(&format!("{{ owner: {caller0}.private, amount: 1u64.private, _nonce: {nonce_b}.public }}"))
                .unwrap();

        (output_a, output_b)
    };

    // Check again to make sure we didn't modify the authorization before calling `evaluate`.
    assert_eq!(authorization.len(), 5);

    // Compute the output value.
    let response = process.evaluate::<CurrentAleo>(authorization.replicate()).unwrap();
    let candidate = response.outputs();
    assert_eq!(2, candidate.len());
    assert_eq!(output_a, candidate[0]);
    assert_eq!(output_b, candidate[1]);

    // Check again to make sure we didn't modify the authorization after calling `evaluate`.
    assert_eq!(authorization.len(), 5);

    // Execute the request.
    let (response, _trace) = process.execute::<CurrentAleo>(authorization).unwrap();
    let candidate = response.outputs();
    assert_eq!(2, candidate.len());
    assert_eq!(output_a, candidate[0]);
    assert_eq!(output_b, candidate[1]);

    // use circuit::Environment;
    //
    // assert_eq!(6427, CurrentAleo::num_constants());
    // assert_eq!(8, CurrentAleo::num_public());
    // assert_eq!(21264, CurrentAleo::num_private());
    // assert_eq!(21279, CurrentAleo::num_constraints());
    // assert_eq!(81872, CurrentAleo::num_gates());
    //
    // assert_eq!(18504, CurrentAleo::num_constants());
    // assert_eq!(17, CurrentAleo::num_public());
    // assert_eq!(58791, CurrentAleo::num_private());
    // assert_eq!(58855, CurrentAleo::num_constraints());
    // assert_eq!(215810, CurrentAleo::num_gates());
}

#[test]
fn test_process_execute_and_finalize_get_add_set() {
    // Initialize a new program.
    let (string, program) = Program::<CurrentNetwork>::parse(
        r"
program testing.aleo;

mapping account:
    key owner as address.public;
    value amount as u64.public;

function compute:
    input r0 as address.public;
    input r1 as u64.public;
    input r2 as u64.public;
    add r1 r2 into r3;
    finalize r0 r3;

finalize compute:
    input r0 as address.public;
    input r1 as u64.public;
    get.or_use account[r0] 0u64 into r2;
    add r2 r1 into r3;
    set r3 into account[r0];
",
    )
    .unwrap();
    assert!(string.is_empty(), "Parser did not consume all of the string: '{string}'");

    // Declare the program ID.
    let program_id = program.id();
    // Declare the mapping.
    let mapping_name = Identifier::from_str("account").unwrap();
    // Declare the function name.
    let function_name = Identifier::from_str("compute").unwrap();

    // Initialize the RNG.
    let rng = &mut TestRng::default();

    // Construct the process.
    let process = super::test_helpers::sample_process(&program);
    // Check that the circuit key can be synthesized.
    process.synthesize_key::<CurrentAleo, _>(program.id(), &function_name, rng).unwrap();

    // Reset the process.
    let mut process = Process::load().unwrap();

    // Initialize a new block store.
    let block_store = BlockStore::<_, BlockMemory<_>>::open(None).unwrap();
    // Initialize a new finalize store.
    let finalize_store = FinalizeStore::<_, FinalizeMemory<_>>::open(None).unwrap();

    // Add the program to the process.
    let deployment = process.deploy::<CurrentAleo, _>(&program, rng).unwrap();
    // Check that the deployment verifies.
    process.verify_deployment::<CurrentAleo, _>(&deployment, rng).unwrap();
    // Finalize the deployment.
    let (stack, _) = process.finalize_deployment(&finalize_store, &deployment).unwrap();
    // Add the stack *manually* to the process.
    process.add_stack(stack);

    // Initialize a new caller account.
    let caller_private_key = PrivateKey::<CurrentNetwork>::new(rng).unwrap();
    let _caller_view_key = ViewKey::try_from(&caller_private_key).unwrap();
    let caller = Address::try_from(&caller_private_key).unwrap();

    // Declare the input value.
    let r0 = Value::<CurrentNetwork>::from_str(&caller.to_string()).unwrap();
    let r1 = Value::<CurrentNetwork>::from_str("3u64").unwrap();
    let r2 = Value::<CurrentNetwork>::from_str("5u64").unwrap();

    // Authorize the function call.
    let authorization = process
        .authorize::<CurrentAleo, _>(&caller_private_key, program.id(), function_name, [r0, r1, r2].iter(), rng)
        .unwrap();
    assert_eq!(authorization.len(), 1);

    // Compute the output value.
    let response = process.evaluate::<CurrentAleo>(authorization.replicate()).unwrap();
    let candidate = response.outputs();
    assert_eq!(0, candidate.len());

    // Check again to make sure we didn't modify the authorization after calling `evaluate`.
    assert_eq!(authorization.len(), 1);

    // Execute the request.
    let (response, mut trace) = process.execute::<CurrentAleo>(authorization).unwrap();
    let candidate = response.outputs();
    assert_eq!(0, candidate.len());

    // Prepare the trace.
    trace.prepare(block_store).unwrap();
    // Prove the execution.
    let execution = trace.prove_execution::<CurrentAleo, _>("testing", rng).unwrap();

    // Verify the execution.
    process.verify_execution(&execution).unwrap();

    // Now, finalize the execution.
    process.finalize_execution(&finalize_store, &execution).unwrap();

    // Check that the account balance is now 8.
    let candidate = finalize_store
        .get_value_speculative(program_id, &mapping_name, &Plaintext::from(Literal::Address(caller)))
        .unwrap()
        .unwrap();
    assert_eq!(candidate, Value::from_str("8u64").unwrap());
}

#[test]
fn test_process_execute_and_finalize_increment_decrement_via_get_set() {
    // Initialize a new program.
    let (string, program) = Program::<CurrentNetwork>::parse(
        r"
program testing.aleo;

mapping account:
    key owner as address.public;
    value amount as u64.public;

function compute:
    input r0 as address.public;
    input r1 as u64.public;
    input r2 as u64.public;
    add r1 r2 into r3;
    finalize r0 r3;

finalize compute:
    input r0 as address.public;
    input r1 as u64.public;
    get.or_use account[r0] 0u64 into r2;
    add r2 r1 into r3;
    sub r3 r1 into r4;
    set r4 into account[r0];
",
    )
    .unwrap();
    assert!(string.is_empty(), "Parser did not consume all of the string: '{string}'");

    // Declare the program ID.
    let program_id = program.id();
    // Declare the mapping.
    let mapping_name = Identifier::from_str("account").unwrap();
    // Declare the function name.
    let function_name = Identifier::from_str("compute").unwrap();

    // Initialize the RNG.
    let rng = &mut TestRng::default();

    // Construct the process.
    let process = super::test_helpers::sample_process(&program);
    // Check that the circuit key can be synthesized.
    process.synthesize_key::<CurrentAleo, _>(program.id(), &function_name, rng).unwrap();

    // Reset the process.
    let mut process = Process::load().unwrap();

    // Initialize a new block store.
    let block_store = BlockStore::<_, BlockMemory<_>>::open(None).unwrap();
    // Initialize a new finalize store.
    let finalize_store = FinalizeStore::<_, FinalizeMemory<_>>::open(None).unwrap();

    // Add the program to the process.
    let deployment = process.deploy::<CurrentAleo, _>(&program, rng).unwrap();
    // Check that the deployment verifies.
    process.verify_deployment::<CurrentAleo, _>(&deployment, rng).unwrap();
    // Finalize the deployment.
    let (stack, _) = process.finalize_deployment(&finalize_store, &deployment).unwrap();
    // Add the stack *manually* to the process.
    process.add_stack(stack);

    // Initialize a new caller account.
    let caller_private_key = PrivateKey::<CurrentNetwork>::new(rng).unwrap();
    let _caller_view_key = ViewKey::try_from(&caller_private_key).unwrap();
    let caller = Address::try_from(&caller_private_key).unwrap();

    // Declare the input value.
    let r0 = Value::<CurrentNetwork>::from_str(&caller.to_string()).unwrap();
    let r1 = Value::<CurrentNetwork>::from_str("3u64").unwrap();
    let r2 = Value::<CurrentNetwork>::from_str("5u64").unwrap();

    // Authorize the function call.
    let authorization = process
        .authorize::<CurrentAleo, _>(&caller_private_key, program.id(), function_name, [r0, r1, r2].iter(), rng)
        .unwrap();
    assert_eq!(authorization.len(), 1);

    // Compute the output value.
    let response = process.evaluate::<CurrentAleo>(authorization.replicate()).unwrap();
    let candidate = response.outputs();
    assert_eq!(0, candidate.len());

    // Check again to make sure we didn't modify the authorization after calling `evaluate`.
    assert_eq!(authorization.len(), 1);

    // Execute the request.
    let (response, mut trace) = process.execute::<CurrentAleo>(authorization).unwrap();
    let candidate = response.outputs();
    assert_eq!(0, candidate.len());

    // Prepare the trace.
    trace.prepare(block_store).unwrap();
    // Prove the execution.
    let execution = trace.prove_execution::<CurrentAleo, _>("testing", rng).unwrap();

    // Verify the execution.
    process.verify_execution(&execution).unwrap();

    // Now, finalize the execution.
    process.finalize_execution(&finalize_store, &execution).unwrap();

    // Check that the account balance is now 0.
    let candidate = finalize_store
        .get_value_speculative(program_id, &mapping_name, &Plaintext::from(Literal::Address(caller)))
        .unwrap()
        .unwrap();
    assert_eq!(candidate, Value::from_str("0u64").unwrap());
}

#[test]
fn test_process_execute_mint_public() {
    // Initialize a new program.
    let (string, program) = Program::<CurrentNetwork>::parse(
        r"
program token.aleo;

// On-chain storage of an `account` map, with `owner` as the key,
// and `amount` as the value.
mapping account:
    // The token owner.
    key owner as address.public;
    // The token amount.
    value amount as u64.public;

// The function `mint_public` issues the specified token amount
// for the token receiver publicly on the network.
function mint_public:
    // Input the token receiver.
    input r0 as address.public;
    // Input the token amount.
    input r1 as u64.public;
    // Mint the tokens publicly.
    finalize r0 r1;

// The finalize scope of `mint_public` increments the
// `account` of the token receiver by the specified amount.
finalize mint_public:
    // Input the token receiver.
    input r0 as address.public;
    // Input the token amount.
    input r1 as u64.public;

    // Get `account[r0]` into `r2`, defaulting to 0u64 if the entry does not exist.
    get.or_use account[r0] 0u64 into r2;
    // Add `r1` to `r2`. If the operation overflows, `mint_public` is reverted.
    add r2 r1 into r3;
    // Set `r3` into `account[r0]`.
    set r3 into account[r0];
",
    )
    .unwrap();
    assert!(string.is_empty(), "Parser did not consume all of the string: '{string}'");

    // Declare the program ID.
    let program_id = program.id();
    // Declare the mapping.
    let mapping_name = Identifier::from_str("account").unwrap();
    // Declare the function name.
    let function_name = Identifier::from_str("mint_public").unwrap();

    // Initialize the RNG.
    let rng = &mut TestRng::default();

    // Construct the process.
    let process = super::test_helpers::sample_process(&program);
    // Check that the circuit key can be synthesized.
    process.synthesize_key::<CurrentAleo, _>(program.id(), &function_name, rng).unwrap();

    // Reset the process.
    let mut process = Process::load().unwrap();

    // Initialize a new block store.
    let block_store = BlockStore::<_, BlockMemory<_>>::open(None).unwrap();
    // Initialize a new finalize store.
    let finalize_store = FinalizeStore::<_, FinalizeMemory<_>>::open(None).unwrap();

    // Add the program to the process.
    let deployment = process.deploy::<CurrentAleo, _>(&program, rng).unwrap();
    // Check that the deployment verifies.
    process.verify_deployment::<CurrentAleo, _>(&deployment, rng).unwrap();
    // Finalize the deployment.
    let (stack, _) = process.finalize_deployment(&finalize_store, &deployment).unwrap();
    // Add the stack *manually* to the process.
    process.add_stack(stack);

    // TODO (howardwu): Remove this. I call this to synthesize the proving key independent of the assignment from 'execute'.
    //  In general, we should update all tests to utilize a presynthesized proving key, before execution, to test
    //  the correctness of the synthesizer.
    process.synthesize_key::<CurrentAleo, _>(program.id(), &function_name, rng).unwrap();

    // Initialize a new caller account.
    let caller_private_key = PrivateKey::<CurrentNetwork>::new(rng).unwrap();
    let _caller_view_key = ViewKey::try_from(&caller_private_key).unwrap();
    let caller = Address::try_from(&caller_private_key).unwrap();

    // Declare the input value.
    let r0 = Value::<CurrentNetwork>::from_str(&caller.to_string()).unwrap();
    let r1 = Value::<CurrentNetwork>::from_str("3u64").unwrap();

    // Authorize the function call.
    let authorization = process
        .authorize::<CurrentAleo, _>(&caller_private_key, program.id(), function_name, [r0, r1].iter(), rng)
        .unwrap();
    assert_eq!(authorization.len(), 1);

    // Compute the output value.
    let response = process.evaluate::<CurrentAleo>(authorization.replicate()).unwrap();
    let candidate = response.outputs();
    assert_eq!(0, candidate.len());

    // Check again to make sure we didn't modify the authorization after calling `evaluate`.
    assert_eq!(authorization.len(), 1);

    // Execute the request.
    let (response, mut trace) = process.execute::<CurrentAleo>(authorization).unwrap();
    let candidate = response.outputs();
    assert_eq!(0, candidate.len());

    // Prepare the trace.
    trace.prepare(block_store).unwrap();
    // Prove the execution.
    let execution = trace.prove_execution::<CurrentAleo, _>("testing", rng).unwrap();

    // Verify the execution.
    process.verify_execution(&execution).unwrap();

    // Now, finalize the execution.
    process.finalize_execution(&finalize_store, &execution).unwrap();

    // Check the account balance.
    let candidate = finalize_store
        .get_value_speculative(program_id, &mapping_name, &Plaintext::from(Literal::Address(caller)))
        .unwrap()
        .unwrap();
    assert_eq!(candidate, Value::from_str("3u64").unwrap());
}

#[test]
fn test_process_execute_call_mint_public() {
    // Initialize a new program.
    let (string, program0) = Program::<CurrentNetwork>::parse(
        r"
program token.aleo;

// On-chain storage of an `account` map, with `owner` as the key,
// and `amount` as the value.
mapping account:
    // The token owner.
    key owner as address.public;
    // The token amount.
    value amount as u64.public;

// The function `mint_public` issues the specified token amount
// for the token receiver publicly on the network.
function mint_public:
    // Input the token receiver.
    input r0 as address.public;
    // Input the token amount.
    input r1 as u64.public;
    // Mint the tokens publicly.
    finalize r0 r1;

// The finalize scope of `mint_public` increments the
// `account` of the token receiver by the specified amount.
finalize mint_public:
    // Input the token receiver.
    input r0 as address.public;
    // Input the token amount.
    input r1 as u64.public;

    // Get `account[r0]` into `r2`, defaulting to 0u64 if the entry does not exist.
    get.or_use account[r0] 0u64 into r2;
    // Add `r1` to `r2`. If the operation overflows, `mint_public` is reverted.
    add r2 r1 into r3;
    // Set `r3` into `account[r0]`.
    set r3 into account[r0];
",
    )
    .unwrap();
    assert!(string.is_empty(), "Parser did not consume all of the string: '{string}'");

    // Declare the mapping.
    let mapping_name = Identifier::from_str("account").unwrap();
    // Declare the function name.
    let function_name = Identifier::from_str("mint_public").unwrap();

    // Initialize the RNG.
    let rng = &mut TestRng::default();

    // Construct the process.
    let process = super::test_helpers::sample_process(&program0);
    // Check that the circuit key can be synthesized.
    process.synthesize_key::<CurrentAleo, _>(program0.id(), &function_name, rng).unwrap();

    // Reset the process.
    let mut process = Process::load().unwrap();

    // Initialize a new block store.
    let block_store = BlockStore::<_, BlockMemory<_>>::open(None).unwrap();
    // Initialize a new finalize store.
    let finalize_store = FinalizeStore::<_, FinalizeMemory<_>>::open(None).unwrap();

    // Add the program to the process.
    let deployment = process.deploy::<CurrentAleo, _>(&program0, rng).unwrap();
    // Check that the deployment verifies.
    process.verify_deployment::<CurrentAleo, _>(&deployment, rng).unwrap();
    // Finalize the deployment.
    let (stack, _) = process.finalize_deployment(&finalize_store, &deployment).unwrap();
    // Add the stack *manually* to the process.
    process.add_stack(stack);

    // TODO (howardwu): Remove this. I call this to synthesize the proving key independent of the assignment from 'execute'.
    //  In general, we should update all tests to utilize a presynthesized proving key, before execution, to test
    //  the correctness of the synthesizer.
    process.synthesize_key::<CurrentAleo, _>(program0.id(), &function_name, rng).unwrap();

    // Initialize another program.
    let (string, program1) = Program::<CurrentNetwork>::parse(
        r"
import token.aleo;

program public_wallet.aleo;

function mint:
    input r0 as address.public;
    input r1 as u64.public;
    call token.aleo/mint_public r0 r1;",
    )
    .unwrap();
    assert!(string.is_empty(), "Parser did not consume all of the string: '{string}'");

    // Add the program to the process.
    process.add_program(&program1).unwrap();

    // Initialize the RNG.
    let rng = &mut TestRng::default();

    // Initialize caller.
    let caller_private_key = PrivateKey::<CurrentNetwork>::new(rng).unwrap();
    let caller = Address::try_from(&caller_private_key).unwrap();

    // Declare the function name.
    let function_name = Identifier::from_str("mint").unwrap();

    // Declare the input value.
    let r0 = Value::<CurrentNetwork>::from_str(&caller.to_string()).unwrap();
    let r1 = Value::<CurrentNetwork>::from_str("100u64").unwrap();

    // Authorize the function call.
    let authorization = process
        .authorize::<CurrentAleo, _>(&caller_private_key, program1.id(), function_name, [r0, r1].iter(), rng)
        .unwrap();
    assert_eq!(authorization.len(), 2);

    // Compute the output value.
    let response = process.evaluate::<CurrentAleo>(authorization.replicate()).unwrap();
    let candidate = response.outputs();
    assert_eq!(0, candidate.len());

    // Check again to make sure we didn't modify the authorization after calling `evaluate`.
    assert_eq!(authorization.len(), 2);

    // Execute the request.
    let (response, mut trace) = process.execute::<CurrentAleo>(authorization).unwrap();
    let candidate = response.outputs();
    assert_eq!(0, candidate.len());

    // Prepare the trace.
    trace.prepare(block_store).unwrap();
    // Prove the execution.
    let execution = trace.prove_execution::<CurrentAleo, _>("testing", rng).unwrap();

    // Verify the execution.
    process.verify_execution(&execution).unwrap();

    // Now, finalize the execution.
    process.finalize_execution(&finalize_store, &execution).unwrap();

    // Check the account balance.
    let candidate = finalize_store
        .get_value_speculative(program0.id(), &mapping_name, &Plaintext::from(Literal::Address(caller)))
        .unwrap()
        .unwrap();
    assert_eq!(candidate, Value::from_str("100u64").unwrap());
}

#[test]
fn test_process_execute_and_finalize_get_set() {
    // Initialize a new program.
    let (string, program) = Program::<CurrentNetwork>::parse(
        r"
program testing.aleo;

mapping account:
    key owner as address.public;
    value amount as u64.public;

function compute:
    input r0 as address.public;
    input r1 as u64.public;
    input r2 as u64.public;
    add r1 r2 into r3;
    finalize r0 r3;

finalize compute:
    input r0 as address.public;
    input r1 as u64.public;
    get.or_use account[r0] 0u64 into r2;
    add r1 r2 into r3;
    set r3 into account[r0];
    get account[r0] into r4;
    add r1 r4 into r5;
    set r5 into account[r0];
",
    )
    .unwrap();
    assert!(string.is_empty(), "Parser did not consume all of the string: '{string}'");

    // Declare the program ID.
    let program_id = program.id();
    // Declare the mapping.
    let mapping_name = Identifier::from_str("account").unwrap();
    // Declare the function name.
    let function_name = Identifier::from_str("compute").unwrap();

    // Initialize the RNG.
    let rng = &mut TestRng::default();

    // Construct the process.
    let process = super::test_helpers::sample_process(&program);
    // Check that the circuit key can be synthesized.
    process.synthesize_key::<CurrentAleo, _>(program.id(), &function_name, rng).unwrap();

    // Reset the process.
    let mut process = Process::load().unwrap();

    // Initialize a new block store.
    let block_store = BlockStore::<_, BlockMemory<_>>::open(None).unwrap();
    // Initialize a new finalize store.
    let finalize_store = FinalizeStore::<_, FinalizeMemory<_>>::open(None).unwrap();

    // Add the program to the process.
    let deployment = process.deploy::<CurrentAleo, _>(&program, rng).unwrap();
    // Check that the deployment verifies.
    process.verify_deployment::<CurrentAleo, _>(&deployment, rng).unwrap();
    // Finalize the deployment.
    let (stack, _) = process.finalize_deployment(&finalize_store, &deployment).unwrap();
    // Add the stack *manually* to the process.
    process.add_stack(stack);

    // Initialize a new caller account.
    let caller_private_key = PrivateKey::<CurrentNetwork>::new(rng).unwrap();
    let _caller_view_key = ViewKey::try_from(&caller_private_key).unwrap();
    let caller = Address::try_from(&caller_private_key).unwrap();

    // Declare the input value.
    let r0 = Value::<CurrentNetwork>::from_str(&caller.to_string()).unwrap();
    let r1 = Value::<CurrentNetwork>::from_str("3u64").unwrap();
    let r2 = Value::<CurrentNetwork>::from_str("5u64").unwrap();

    // Authorize the function call.
    let authorization = process
        .authorize::<CurrentAleo, _>(&caller_private_key, program.id(), function_name, [r0, r1, r2].iter(), rng)
        .unwrap();
    assert_eq!(authorization.len(), 1);

    // Compute the output value.
    let response = process.evaluate::<CurrentAleo>(authorization.replicate()).unwrap();
    let candidate = response.outputs();
    assert_eq!(0, candidate.len());

    // Check again to make sure we didn't modify the authorization after calling `evaluate`.
    assert_eq!(authorization.len(), 1);

    // Execute the request.
    let (response, mut trace) = process.execute::<CurrentAleo>(authorization).unwrap();
    let candidate = response.outputs();
    assert_eq!(0, candidate.len());

    // Prepare the trace.
    trace.prepare(block_store).unwrap();
    // Prove the execution.
    let execution = trace.prove_execution::<CurrentAleo, _>("testing", rng).unwrap();

    // Verify the execution.
    process.verify_execution(&execution).unwrap();

    // Now, finalize the execution.
    process.finalize_execution(&finalize_store, &execution).unwrap();

    // Check that the account balance is now 8.
    let candidate = finalize_store
        .get_value_speculative(program_id, &mapping_name, &Plaintext::from(Literal::Address(caller)))
        .unwrap()
        .unwrap();
    assert_eq!(candidate, Value::from_str("16u64").unwrap());
}

#[test]
fn test_execution_order() {
    // Initialize a new program.
    let (string, program0) = Program::<CurrentNetwork>::parse(
        r"
program zero.aleo;

function c:
    input r0 as u8.private;
    input r1 as u8.private;
    add r0 r1 into r2;
    output r2 as u8.private;",
    )
    .unwrap();
    assert!(string.is_empty(), "Parser did not consume all of the string: '{string}'");

    // Construct the process.
    let mut process = super::test_helpers::sample_process(&program0);

    // Initialize another program.
    let (string, program1) = Program::<CurrentNetwork>::parse(
        r"
import zero.aleo;

program one.aleo;

function b:
    input r0 as u8.private;
    input r1 as u8.private;
    call zero.aleo/c r0 r1 into r2;
    output r2 as u8.private;",
    )
    .unwrap();
    assert!(string.is_empty(), "Parser did not consume all of the string: '{string}'");

    // Add the program to the process.
    process.add_program(&program1).unwrap();

    // Initialize another program.
    let (string, program2) = Program::<CurrentNetwork>::parse(
        r"
import one.aleo;

program two.aleo;

function a:
    input r0 as u8.private;
    input r1 as u8.private;
    call one.aleo/b r0 r1 into r2;
    output r2 as u8.private;",
    )
    .unwrap();
    assert!(string.is_empty(), "Parser did not consume all of the string: '{string}'");

    // Add the program to the process.
    process.add_program(&program2).unwrap();

    // Initialize the RNG.
    let rng = &mut TestRng::default();

    // Initialize the caller.
    let caller_private_key = PrivateKey::<CurrentNetwork>::new(rng).unwrap();

    // Declare the function name.
    let function_name = Identifier::from_str("a").unwrap();

    // Declare the input value.
    let r0 = Value::<CurrentNetwork>::from_str("1u8").unwrap();
    let r1 = Value::<CurrentNetwork>::from_str("2u8").unwrap();

    // Authorize the function call.
    let authorization = process
        .authorize::<CurrentAleo, _>(&caller_private_key, program2.id(), function_name, [r0, r1].iter(), rng)
        .unwrap();
    assert_eq!(authorization.len(), 3);
    println!("\nAuthorize\n{:#?}\n\n", authorization.to_vec_deque());

    let output = Value::<CurrentNetwork>::from_str("3u8").unwrap();

    // Compute the output value.
    let response = process.evaluate::<CurrentAleo>(authorization.replicate()).unwrap();
    let candidate = response.outputs();
    assert_eq!(1, candidate.len());
    assert_eq!(output, candidate[0]);

    // Check again to make sure we didn't modify the authorization after calling `evaluate`.
    assert_eq!(authorization.len(), 3);

    // Execute the request.
    let (response, execution, _inclusion, _metrics) = process.execute::<CurrentAleo, _>(authorization, rng).unwrap();
    let candidate = response.outputs();
    assert_eq!(1, candidate.len());
    assert_eq!(output, candidate[0]);

    process.verify_execution::<false>(&execution).unwrap();
}

#[test]
fn test_complex_execution_order() {
    // This test checks that the execution order is correct.
    // The functions are invoked in the following order:
    // "four::a"
    //   --> "two::b"
    //        --> "zero::c"
    //        --> "one::d"
    //   --> "three::e"
    //        --> "two::b"
    //             --> "zero::c"
    //             --> "one::d"
    //        --> "one::d"
    //        --> "zero::c"
    // The linearized order is:
    //  - [a, b, c, d, e, b, c, d, d, c]
    // The transitions must be included in the `Execution` in the order they finish.
    // The execution order is:
    //  - [c, d, b, c, d, b, d, c, e, a]

    // Initialize a new program.
    let (string, program0) = Program::<CurrentNetwork>::parse(
        r"
    program zero.aleo;

    function c:
        input r0 as u8.private;
        input r1 as u8.private;
        add r0 r1 into r2;
        output r2 as u8.private;",
    )
    .unwrap();
    assert!(string.is_empty(), "Parser did not consume all of the string: '{string}'");

    // Construct the process.
    let mut process = super::test_helpers::sample_process(&program0);

    // Initialize another program.
    let (string, program1) = Program::<CurrentNetwork>::parse(
        r"
    program one.aleo;

    function d:
        input r0 as u8.private;
        input r1 as u8.private;
        add r0 r1 into r2;
        output r2 as u8.private;",
    )
    .unwrap();
    assert!(string.is_empty(), "Parser did not consume all of the string: '{string}'");

    // Add the program to the process.
    process.add_program(&program1).unwrap();

    // Initialize another program.
    let (string, program2) = Program::<CurrentNetwork>::parse(
        r"
    import zero.aleo;
    import one.aleo;

    program two.aleo;

    function b:
        input r0 as u8.private;
        input r1 as u8.private;
        call zero.aleo/c r0 r1 into r2;
        call one.aleo/d r1 r2 into r3;
        output r3 as u8.private;",
    )
    .unwrap();
    assert!(string.is_empty(), "Parser did not consume all of the string: '{string}'");

    // Add the program to the process.
    process.add_program(&program2).unwrap();

    // Initialize another program.
    let (string, program3) = Program::<CurrentNetwork>::parse(
        r"
    import zero.aleo;
    import one.aleo;
    import two.aleo;

    program three.aleo;

    function e:
        input r0 as u8.private;
        input r1 as u8.private;
        call two.aleo/b r0 r1 into r2;
        call one.aleo/d r1 r2 into r3;
        call zero.aleo/c r1 r2 into r4;
        output r4 as u8.private;",
    )
    .unwrap();
    assert!(string.is_empty(), "Parser did not consume all of the string: '{string}'");

    // Add the program to the process.
    process.add_program(&program3).unwrap();

    // Initialize another program.
    let (string, program4) = Program::<CurrentNetwork>::parse(
        r"
    import two.aleo;
    import three.aleo;

    program four.aleo;

    function a:
        input r0 as u8.private;
        input r1 as u8.private;
        call two.aleo/b r0 r1 into r2;
        call three.aleo/e r1 r2 into r3;
        output r3 as u8.private;",
    )
    .unwrap();
    assert!(string.is_empty(), "Parser did not consume all of the string: '{string}'");

    // Add the program to the process.
    process.add_program(&program4).unwrap();

    // Initialize the RNG.
    let rng = &mut TestRng::default();

    // Initialize caller.
    let caller_private_key = PrivateKey::<CurrentNetwork>::new(rng).unwrap();

    // Declare the function name.
    let function_name = Identifier::from_str("a").unwrap();

    // Declare the input value.
    let r0 = Value::<CurrentNetwork>::from_str("1u8").unwrap();
    let r1 = Value::<CurrentNetwork>::from_str("2u8").unwrap();

    // Authorize the function call.
    let authorization = process
        .authorize::<CurrentAleo, _>(&caller_private_key, program4.id(), function_name, [r0, r1].iter(), rng)
        .unwrap();
    assert_eq!(authorization.len(), 10);
    println!("\nAuthorize\n{:#?}\n\n", authorization.to_vec_deque());

    let output = Value::<CurrentNetwork>::from_str("17u8").unwrap();

    // Compute the output value.
    let response = process.evaluate::<CurrentAleo>(authorization.replicate()).unwrap();
    let candidate = response.outputs();
    assert_eq!(1, candidate.len());
    assert_eq!(output, candidate[0]);

    // Check again to make sure we didn't modify the authorization after calling `evaluate`.
    assert_eq!(authorization.len(), 10);

    // Execute the request.
    let (response, execution, _inclusion, _metrics) = process.execute::<CurrentAleo, _>(authorization, rng).unwrap();
    let candidate = response.outputs();
    assert_eq!(1, candidate.len());
    assert_eq!(output, candidate[0]);

    // Construct the expected execution order.
    let expected_order = [
        (program0.id(), Identifier::<Testnet3>::from_str("c").unwrap()),
        (program1.id(), Identifier::<Testnet3>::from_str("d").unwrap()),
        (program2.id(), Identifier::<Testnet3>::from_str("b").unwrap()),
        (program0.id(), Identifier::<Testnet3>::from_str("c").unwrap()),
        (program1.id(), Identifier::<Testnet3>::from_str("d").unwrap()),
        (program2.id(), Identifier::<Testnet3>::from_str("b").unwrap()),
        (program1.id(), Identifier::<Testnet3>::from_str("d").unwrap()),
        (program0.id(), Identifier::<Testnet3>::from_str("c").unwrap()),
        (program3.id(), Identifier::<Testnet3>::from_str("e").unwrap()),
        (program4.id(), Identifier::<Testnet3>::from_str("a").unwrap()),
    ];
    for (transition, (expected_program_id, expected_function_name)) in
        execution.transitions().zip_eq(expected_order.iter())
    {
        assert_eq!(transition.program_id(), *expected_program_id);
        assert_eq!(transition.function_name(), expected_function_name);
    }
}

#[test]
fn test_process_execute_and_finalize_get_set_with_struct() {
    // Initialize a new program.
    let (string, program) = Program::<CurrentNetwork>::parse(
        r"
program testing.aleo;

struct entry:
    count as u8;
    data as u8;

mapping entries:
    key owner as address.public;
    value entry as entry.public;

function compute:
    input r0 as u8.public;
    input r1 as u8.public;
    cast r0 r1 into r2 as entry;
    finalize self.caller r2;

finalize compute:
    input r0 as address.public;
    input r1 as entry.public;
    get.or_use entries[r0] r1 into r2;
    add r1.count r2.count into r3;
    add r1.data r2.data into r4;
    cast r3 r4 into r5 as entry;
    set r5 into entries[r0];
    get entries[r0] into r6;
    add r6.count r1.count into r7;
    add r6.data r1.data into r8;
    cast r7 r8 into r9 as entry;
    set r9 into entries[r0];
",
    )
    .unwrap();
    assert!(string.is_empty(), "Parser did not consume all of the string: '{string}'");

    // Declare the program ID.
    let program_id = program.id();
    // Declare the mapping.
    let mapping_name = Identifier::from_str("entries").unwrap();
    // Declare the function name.
    let function_name = Identifier::from_str("compute").unwrap();

    // Initialize the RNG.
    let rng = &mut TestRng::default();

    // Construct the process.
    let process = super::test_helpers::sample_process(&program);
    // Check that the circuit key can be synthesized.
    process.synthesize_key::<CurrentAleo, _>(program.id(), &function_name, rng).unwrap();

    // Reset the process.
    let mut process = Process::load().unwrap();

    // Initialize a new block store.
    let block_store = BlockStore::<_, BlockMemory<_>>::open(None).unwrap();
    // Initialize a new finalize store.
    let finalize_store = FinalizeStore::<_, FinalizeMemory<_>>::open(None).unwrap();

    // Add the program to the process.
    let deployment = process.deploy::<CurrentAleo, _>(&program, rng).unwrap();
    // Check that the deployment verifies.
    process.verify_deployment::<CurrentAleo, _>(&deployment, rng).unwrap();
    // Finalize the deployment.
    let (stack, _) = process.finalize_deployment(&finalize_store, &deployment).unwrap();
    // Add the stack *manually* to the process.
    process.add_stack(stack);

    // Initialize a new caller account.
    let caller_private_key = PrivateKey::<CurrentNetwork>::new(rng).unwrap();
    let _caller_view_key = ViewKey::try_from(&caller_private_key).unwrap();
    let caller = Address::try_from(&caller_private_key).unwrap();

    // Declare the input value.
    let r0 = Value::<CurrentNetwork>::from_str("1u8").unwrap();
    let r1 = Value::<CurrentNetwork>::from_str("2u8").unwrap();

    // Authorize the function call.
    let authorization = process
        .authorize::<CurrentAleo, _>(&caller_private_key, program.id(), function_name, [r0, r1].iter(), rng)
        .unwrap();
    assert_eq!(authorization.len(), 1);

    // Compute the output value.
    let response = process.evaluate::<CurrentAleo>(authorization.replicate()).unwrap();
    let candidate = response.outputs();
    assert_eq!(0, candidate.len());

    // Check again to make sure we didn't modify the authorization after calling `evaluate`.
    assert_eq!(authorization.len(), 1);

    // Execute the request.
    let (response, mut trace) = process.execute::<CurrentAleo>(authorization).unwrap();
    let candidate = response.outputs();
    assert_eq!(0, candidate.len());

    // Prepare the trace.
    trace.prepare(block_store).unwrap();
    // Prove the execution.
    let execution = trace.prove_execution::<CurrentAleo, _>("testing", rng).unwrap();

    // Verify the execution.
    process.verify_execution(&execution).unwrap();

    // Now, finalize the execution.
    process.finalize_execution(&finalize_store, &execution).unwrap();

    // Check that the struct is stored as expected.
    let candidate = finalize_store
        .get_value_speculative(program_id, &mapping_name, &Plaintext::from(Literal::Address(caller)))
        .unwrap()
        .unwrap();
    assert_eq!(candidate, Value::from_str("{ count: 3u8, data: 6u8 }").unwrap());
}

#[test]
fn test_sanity_check_transfer_and_fee() {
    use console::types::Group;

    // Initialize the RNG.
    let rng = &mut TestRng::default();

    // Initialize a new caller account.
    let private_key = PrivateKey::<CurrentNetwork>::new(rng).unwrap();
    let caller = Address::try_from(&private_key).unwrap();

    // Construct a new process.
    let process = Process::load().unwrap();
    // Retrieve the stack.
    let stack = process.get_stack(ProgramID::from_str("credits.aleo").unwrap()).unwrap();

    /* Transfer */
    {
        // Declare the function name.
        let function_name = Identifier::from_str("transfer_private").unwrap();

        // Declare the inputs.
        let r0 = Value::from_str(&format!(
            "{{ owner: {caller}.private, microcredits: 1_500_000_000_000_000_u64.private, _nonce: {}.public }}",
            Group::<CurrentNetwork>::zero()
        ))
        .unwrap();
        let r1 = Value::<CurrentNetwork>::from_str(&format!("{caller}")).unwrap();
        let r2 = Value::<CurrentNetwork>::from_str("1_500_000_000_000_000_u64").unwrap();

        // Compute the assignment.
        let assignment = get_assignment(stack, &private_key, function_name, &[r0, r1, r2], rng);
        assert_eq!(12, assignment.num_public());
        assert_eq!(54672, assignment.num_private());
        assert_eq!(54730, assignment.num_constraints());
        assert_eq!((88496, 130675, 83625), assignment.num_nonzeros());
    }

    println!();

    /* Fee */
    {
        // Declare the function name.
        let function_name = Identifier::from_str("fee").unwrap();

        // Declare the inputs.
        let r0 = Value::from_str(&format!(
            "{{ owner: {caller}.private, microcredits: 1_500_000_000_000_000_u64.private, _nonce: {}.public }}",
            Group::<CurrentNetwork>::zero()
        ))
        .unwrap();
        let r1 = Value::<CurrentNetwork>::from_str("1_500_000_000_000_000_u64").unwrap();
        let r2 = Value::<CurrentNetwork>::from_str(&Field::<CurrentNetwork>::rand(rng).to_string()).unwrap();

        // Compute the assignment.
        let assignment = get_assignment(stack, &private_key, function_name, &[r0, r1, r2], rng);
        assert_eq!(10, assignment.num_public());
        assert_eq!(41218, assignment.num_private());
        assert_eq!(41265, assignment.num_constraints());
        assert_eq!((64422, 92898, 62357), assignment.num_nonzeros());
    }
}

fn get_assignment(
    stack: &Stack<CurrentNetwork>,
    private_key: &PrivateKey<CurrentNetwork>,
    function_name: Identifier<CurrentNetwork>,
    inputs: &[Value<CurrentNetwork>],
    rng: &mut TestRng,
) -> Assignment<Fr> {
    // Retrieve the program.
    let program = stack.program();
    // Retrieve the input types.
    let input_types = program.get_function(&function_name).unwrap().input_types();
    // Compute the request.
    let request = Request::sign(private_key, *program.id(), function_name, inputs.iter(), &input_types, rng).unwrap();
    // Initialize the assignments.
    let assignments = Assignments::<CurrentNetwork>::default();
    // Initialize the call stack.
    let call_stack = CallStack::CheckDeployment(vec![request], *private_key, assignments.clone());
    // Synthesize the circuit.
    let _response = stack.execute_function::<CurrentAleo>(call_stack).unwrap();
    // Retrieve the assignment.
    let assignment = assignments.read().last().unwrap().clone();
    assignment
}
