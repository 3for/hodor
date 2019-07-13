use crate::air::*;
use ff::PrimeField;
use super::{IntoARP, ARP};
use crate::domains::Domain;
use crate::SynthesisError;



impl<F: PrimeField> ARP<F> {
    pub(crate) fn new<I: IntoARP<F>>(f: I) -> Self {
        f.into_arp()
    }

    /// - make interpolating polynomial f
    /// - add masking coefficients for constraints
    /// - keep boundary constraints as it is
    pub fn route(&mut self ) -> Result<(), SynthesisError> {
        // TODO
        // let witness = self.witness.expect("is something");

        let num_registers = self.num_registers as u64;
        let num_steps = self.num_steps as u64;

        let num_registers_sup = num_registers.next_power_of_two();
        let num_steps_sup = num_steps.next_power_of_two();

        let witness_domain = Domain::<F>::new_for_size(num_registers_sup * num_steps_sup)?;
        println!("Row mask base = {}", witness_domain.generator);
        let steps_domain = Domain::<F>::new_for_size(num_steps_sup)?;
        println!("Column mask base = {}", steps_domain.generator);

        fn remap_univariate_term<F: PrimeField>(
            term: &mut UnivariateTerm<F>,
            row_domain: &Domain<F>,
            column_domain: &Domain<F>,
        ) {
            let register_delta = match term.register {
                Register::Register(num) => {
                    num as u64
                },
                _ => {
                    unreachable!("Registers are now indistinguishable");
                }
            };

            let step_delta = match term.steps_difference {
                StepDifference::Steps(num) => {
                    num as u64
                },
                _ => {
                    unreachable!("Step differences are not masks yet");
                }
            };

            let mut mask = column_domain.generator.pow([step_delta]);
            let per_row_contrib = row_domain.generator.pow([register_delta]);

            mask.mul_assign(&per_row_contrib);

            term.steps_difference = StepDifference::Mask(mask);
        }

        fn remap_term<F: PrimeField>(
            term: &mut ConstraintTerm<F>,
            row_domain: &Domain<F>,
            column_domain: &Domain<F>,
        ) {
            match term {
                ConstraintTerm::Univariate(ref mut t) => {
                    remap_univariate_term(
                        t, 
                        &row_domain,
                        &column_domain
                    );
                },
                ConstraintTerm::Polyvariate(ref mut poly_term) => {
                    for mut t in poly_term.terms.iter_mut() {
                        remap_univariate_term(
                            &mut t, 
                            &row_domain,
                            &column_domain
                        );
                    }
                }
            }
        }

        fn remap_constraint<F: PrimeField>(
            constraint: &mut Constraint<F>,
            row_domain: &Domain<F>,
            column_domain: &Domain<F>,
        ) {
            for mut t in constraint.terms.iter_mut() {
                remap_term(
                    &mut t, 
                    &row_domain,
                    &column_domain
                );
            }
        }

        for mut c in self.constraints.iter_mut() {
            remap_constraint(
                &mut c, 
                &witness_domain,
                &steps_domain
            );
        }

        Ok(())
    }
}

#[test]
fn test_fib_conversion() {
    use crate::Fr;
    use crate::air::Fibonacci;
    use crate::air::TestTraceSystem;
    use crate::air::IntoAIR;
    use crate::arp::IntoARP;

    let fib = Fibonacci::<Fr> {
        first_a: Some(1),
        first_b: Some(1),
        final_a: Some(3),
        at_step: Some(2),
        _marker: std::marker::PhantomData
    };

    let mut test_tracer = TestTraceSystem::<Fr>::new();
    fib.trace(&mut test_tracer).expect("should work");
    test_tracer.calculate_witness(1, 1, 5);
    let mut arp = ARP::<Fr>::new(test_tracer);
    arp.route().expect("must work");

    for c in arp.constraints.iter() {
        println!("{:?}", c);
    }
}
