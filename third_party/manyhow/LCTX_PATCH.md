# manyhow 0.11.4 expression compatibility

The three `bail!` arms expand to block expressions. Their early return, error
conversion and caller spans are unchanged. Inner statement semicolons no longer
appear as trailing semicolons in expression position (Rust issue 79813).

This keeps the locked analyzer dependency family unchanged. Remove the local
patch when its upstream replacement provides this expression behavior.

Focused controls exercise all three arms in expression position with the compiler
lint denied. The upstream infallible-handler test keeps a narrow unreachable-code
expectation for its intentionally impossible error arm.
