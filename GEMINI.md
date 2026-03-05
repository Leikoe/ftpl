# Mandates for FTPL Development

1. **Production-Grade Engineering**: This library is NOT a toy. It is intended for integration into production-grade deep learning compilers (e.g., XLA, Triton, LLVM-based engines).
2. **Mathematical Rigor**: All judgments (contiguity, aliasing, etc.) must be based on structural properties and formal proofs derived from the theory, never on heuristics or "simulated" evaluations.
3. **No Toy Heuristics**: Heuristics that "usually work" are unacceptable. If a property cannot be proven structurally, it must return `Unknown` or a conservative lower bound.
4. **Hardware Alignment**: Every layout transformation must be analyzed for its physical cost (Div/Mod, Bank Conflicts, Vector Width).
