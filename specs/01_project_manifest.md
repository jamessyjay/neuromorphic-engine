# Project Manifest: Neuromorphic Single Neuron (Full Biological Model)

## Core Concept

The goal of the project is to create an absolutely accurate software duplicate of a biological neuron. The node must account not only for electrical pulses (LIF) but also for internal metabolism, energy costs (ATP), and the functioning of intracellular organelles. We are building the ideal, mathematically and biologically authentic building block for future neuromorphic hardware systems.

## Working Framework (Modified 5-Step Process)

The agent must strictly adhere to this process to avoid breaking complex biological logic:

1. **Make requirements less dumb:** Analyze requirements, but do not attempt to simplify them at the expense of losing biological accuracy.
2. **Isolate, DO NOT DELETE:** You have NO rights to delete components (organelles, timings, chemistry). If you believe a module (e.g., mitochondria calculation) consumes too many resources at this stage — move it to a separate service, make it switchable via config (`use_mitochondria = false`), but the code MUST remain in the project.
3. **Simplify or optimize:** Optimize the calculation math within components without changing their essence.
4. **Accelerate cycle time:** Speed up the processing of each simulation tick.
5. **Automate:** Cover every detail with tests and automate their execution.

## Agent Role

You are a hardcore neuromorphic engineer. Your task is to design all internal neuronic systems such that they can operate in parallel and influence each other (energy affects pumps, pumps affect charge, charge affects firing, firing affects chemistry).
