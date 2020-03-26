# Fawkes-Crypto - zkSNARKs building framework


## Abstract

Fawkes-Crypto is a lightweight framework for building circuits in bellman, using groth16 proving system and BN254 curve.

The framework is targeted to use best practices for circuit building from circom and sapling-crypto. 

## Example

Here is an example, how Merkle tree implementation is working.

```rust
pub fn poseidon_merkle_root<E:Engine, CS:ConstraintSystem<E>>(
    mut cs:CS, leaf:&Signal<E>, 
    sibling:&[Signal<E>], 
    path:&[Signal<E>], 
    params:&PoseidonParams<E::Fr>
) -> Result<Signal<E>, SynthesisError> {
    assert!(sibling.len() == path.len(), "merkle proof length should be the same");
    let mut root = leaf.clone();
    let mut i = 0;
    for (p, s) in path.iter().zip(sibling.iter()) {
        i+=1;
        let first = s.switch(cs.namespace(|| format!("sel[{}]", i)), p, &root)?; 
        let second = &root + s - &first;
        root = poseidon(cs.namespace(|| format!("r[{}]", i)), [first, second].as_ref(), params)?;
    }
    Ok(root)
}

```

`Signal` is a sparse linear combination of inputs, based on ordered linked list, so we perform arithmetics with `Signal` with `U(N)` complexity. With `Signal` bellman will allocate additional inputs only when you really need it (for example, in the case when you multiply two nonconstant `Signal`). If you perform multiplication with constant or zero `Signal`, no additional inputs will be allocated.

## Benchmarks

| Circuit | Constraints | Per bit | 
| - | - | - |
| poseidon hash (4, 8, 54) | 255 | 0.33 |
| jubjub oncurve+subgroup check | 16 | |
| ecmul_const 254 bits | 513 | 2.02 |
| ecmul 254 bits | 2296 | 9.04 |
| poseidon merkle proof 32| 7328 | |
| poseidon eddsa | 3860 | |
| rollup 1024 txs, 2^32 set | 35695616 |

At i9-9900K rollup is proved for 628 seconds

## Circuit improvements

* We are using indeterministic subgroup checks, performing most part of computations as witness-only and perform cofactor multiplication at the circuit.
* ecmul and ecmul_cost operations are working assuming that the base point is in the subgroup. This allows us to use Montgomery (0, 0) point as adder initial state. Then the adder never reaches zero point and subgroup point, because (0, 0) is not in subgroup and we can use cheap montgomery_add circuit safely.

See more as ethresear.ch

## Authors

Igor Gulamov

## Disclaimer

Fawkes-Crypto has not been audited and is provided as is, use at your own risk.

## License

Fawkes-Crypto is available under Apache License 2.0 license or MIT as your choice.

