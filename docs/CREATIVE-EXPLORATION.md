# Creative exploration

Use this optional workflow when a consequential human-facing decision has several credible answers and the preferred direction is still unclear. It is intended for occasional work on interfaces, reports, command output, documentation, examples, and product language.

The basic sequence is:

1. **Diverge:** produce a few intentionally different candidates against the same brief.
2. **Converge:** make a human decision, select one branch, and continue iterating there.

Parallel candidates help discover a direction. A continuing implementation thread is usually better once the direction is known.

## Direction uncertainty or execution uncertainty?

Start by naming the uncertainty.

- **Direction uncertainty:** several substantially different answers could satisfy the requirements. Explore alternatives.
- **Execution uncertainty:** the intended result is already clear and the current attempt needs refinement. Stay on one branch and iterate.

Spacing cleanup, accessibility repair, clearer validation messages, and responsive corrections are usually execution work. Choosing the form of a plan report, operator workflow, onboarding path, or diagnostic explanation may benefit from directional exploration.

## Use this selectively

Parallel exploration is useful when:

- the decision is visible or repeatedly encountered;
- several valid treatments are plausible;
- seeing alternatives could change the decision;
- candidate work is cheap enough to discard;
- human judgment carries substantial value.

A single implementation is usually enough when requirements determine most of the answer, the direction is settled, or alternatives would differ only cosmetically.

## 1. Separate fixed requirements from open decisions

Record the requirements every candidate must preserve:

- domain and safety rules;
- privilege and ownership boundaries;
- required evidence and failure behavior;
- accessibility and platform requirements;
- performance and compatibility constraints;
- established project decisions.

Then list the actual questions:

- How should information be grouped?
- Which details belong in the default view?
- How should uncertainty and refusal be explained?
- What should human output emphasize versus JSON output?
- Which language makes an operator decision easiest?
- How much density is appropriate?

A useful exploration answers named questions rather than producing unrelated redesigns.

## 2. Gather a small reference set

Use a few relevant examples from shipped tools, existing repository output, documentation, screenshots, or previous experiments. Record the particular quality being examined.

Also record anti-references: patterns that conflict with this product, such as decorative dashboards, hidden destructive consequences, vague success language, overly friendly security warnings, or dense output without an obvious decision path.

References provide vocabulary. Candidates still need to solve the problem under this repository’s rules.

## 3. Choose the amount of divergence

Use the cheapest level that supports a real decision.

- **Small variation:** alternatives for labels, ordering, grouping, density, or disclosure.
- **Directional variation:** candidates optimize for continuity, reduction, operator guidance, expert density, or auditability.
- **Conceptual variation:** candidates use different interaction or report models.

Broad exploration belongs early. Later work benefits from smaller variants around an accepted direction.

## 4. Give each candidate a reason to exist

Three candidates are a useful default. Possible assignments:

- **Continuity:** improve the current presentation while preserving its existing language.
- **Reduction:** show only what the operator needs for the immediate decision, with deeper evidence available on request.
- **Audit-first:** make identity, evidence, planned action, privilege lane, rollback class, and uncertainty highly visible.

Candidates should work independently during the first pass. Early exposure to the other attempts encourages premature convergence.

## 5. Require comparable evidence

Review every candidate against the same material:

- identical fixture or host-observation data;
- the same success, absent, unknown, conflicting, and failure cases;
- matching terminal widths or viewport sizes;
- the same build and verification expectations;
- a branch and exact commit;
- captured human and JSON output where relevant;
- a short account of decisions and compromises.

Interactive work should include a working preview or recording. Command and report work should include exact fixture inputs and complete output.

## 6. Review by decision

Compare individual decisions rather than immediately selecting a whole candidate:

- Which version makes the next operator decision clearest?
- Which preserves evidence without overwhelming the primary view?
- Which communicates unknown state honestly?
- Which makes privilege and destructive consequences visible?
- Which remains useful at narrow terminal widths?
- Which language could be misread during an incident?
- Which ideas are specific to SmolRunner rather than generic CLI conventions?

Agent review can check the brief, omissions, accessibility, consistency, and safety language. Human review owns the final presentation decision.

## 7. Converge explicitly

Write a concrete selection, for example:

> Use B’s action summary and A’s evidence table. Keep the existing status vocabulary. Reject C’s collapsed uncertainty details. Add B’s narrow-terminal treatment during convergence.

Choose one canonical branch and give the convergence pass:

- the original brief;
- accepted elements;
- rejected directions and reasons;
- unresolved details;
- verification requirements.

Continue there instead of reopening broad alternatives for every small choice.

## 8. Stop diverging

Return to one implementation when the direction can be described clearly, new candidates mostly rearrange details, the same qualities keep winning, or remaining concerns can be written as specific edits.

## Fidelity ladder

Choose the lowest-cost artifact that supports the decision:

1. written direction;
2. sample terminal output or report fixture;
3. static HTML or screenshot;
4. isolated renderer or component;
5. working branch;
6. complete interactive prototype.

Use working code when terminal behavior, responsive layout, state transitions, keyboard interaction, or exact rendering determines quality.

## Lightweight record

```md
# Creative exploration

## Decision to make

## Fixed requirements
- 

## Open questions
- 

## References and anti-references
- 

## Candidates
- A: continuity
- B: reduction
- C: audit-first

## Required fixtures and evidence
- 

## Selection
- Keep:
- Combine:
- Reject:
- Explore during convergence:

## Canonical branch and commit

## Remaining questions
- 
```

## Possible uses in SmolRunner

This workflow may be useful for:

- human-readable `doctor`, `plan`, and future `apply` output;
- presentation of present, absent, unknown, adoptable, foreign, and conflicting resources;
- privilege-lane and rollback explanations;
- installation and first-run guidance;
- recovery and partial-failure reports;
- documentation diagrams and operator examples;
- any future browser or fleet overview.

Keep ownership evidence, canonical locators, privilege rules, command allowlists, redaction, rollback semantics, and mutation preconditions fixed across candidates. Explore how those facts are ordered, summarized, and disclosed.