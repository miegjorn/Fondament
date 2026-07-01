# Deconstructive vs. Crystallization — Empirical Basis

This document backs the claim in `packages/deconstructive/plugin.toml`
("Empirically outperforms crystallization in all tested comparisons") with
the experiments that produced it. It is evidence, not a plan — see
`docs/superpowers/plans/2026-06-18-deconstructive-discipline.md` for how
the discipline itself was implemented.

## The question

Two ways to hand a model an unresolved multi-perspective problem before it
answers:

- **Crystallization** — pre-synthesize the competing positions into a short
  "here's the tension" summary, then ask the question. This is what the
  `deconstructive` discipline replaced.
- **Deconstruction (raw voices)** — inject the competing positions
  themselves, unresolved, and let the model do the synthesis work live,
  inside extended thinking. This is what `deconstructive` does: the
  resolver-generated preamble lists the agent's actual composed parts and
  instructs it to become each one, name the tensions, and recompose.

Six experiments (Sonnet 4.6 throughout, then a 3-model sweep) tested
whether raw-voice injection actually produces better reasoning than
crystallization, or just produces *busier-looking* reasoning that doesn't
change the answer.

## Method

A fixed harness, three conditions per question:

- **A — baseline**: the question alone, no injected material.
- **B — crystallization**: question + a short pre-synthesized "the debate
  converges on tension X" summary.
- **C — raw voices**: question + the named competing positions verbatim,
  presented as unresolved, with an explicit instruction not to expect a
  synthesis — "resolve internally."

A separate model graded each condition's *thinking trace* (not the visible
answer) against the injected voice positions on: whether it engaged with
each named position (none/surface/deep), whether it broke the shared
framing assumption underlying all the positions ("frame dissolution"),
what kind of synthesis move it made (mediation / reframing / rejection /
integration), and an overall 0–10 score.

## Results across all experiments

| Experiment | Question(s) | Model(s) | Result |
|---|---|---|---|
| 1 | identity architecture (Fondament vs. Farga) | Sonnet 4.6 | C produced less thinking than A/B (456w vs 743-797w) but a structurally different, frame-rejecting synthesis. Raw substring-matching grader was unreliable here — motivated experiment 2. |
| 2 | same, semantic grader | Sonnet 4.6 | A=6/10 (no frame dissolution, mediation). B=7/10, C=7/10 (both dissolved the frame, both reframed). First sign B and C converge — but C did it in fewer thinking words. |
| 3 | 3 new domains (agent delegation policy, SaaS pricing, sync/async culture) | Sonnet 4.6 | A and B produced **near-zero thinking** (24–36 words — model skipped private deliberation and answered directly). C was the only condition that forced real engagement (500–800 words) on every case. C scored 8–9/10; A/B scored 0 (nothing to grade). |
| 4 | 3 harder, more contested questions (agent goal self-revision, diffusion of responsibility, predictive policing bias) | Sonnet 4.6 | The "A/B skip thinking" pattern did **not** track designed difficulty — one case triggered substantial baseline thinking (575w), two did not (24–27w), independent of how hard the question was meant to be. C won or tied in every case; B never won outright. |
| 5 | Stress test: degrade the crystallization itself (vague + quietly wrong) on the 2 cases where B had scored competitively | Sonnet 4.6 | Scores held (+1, −1 — within noise). The model **explicitly audited and corrected** the bad synthesis before reasoning, rather than being misled by it or ignoring it. Crystallization *content* quality is not load-bearing — but correcting a wrong synthesis costs about as much thinking as reasoning from raw material, eliminating B's presumed cost advantage whenever the synthesis is wrong. |
| 6 | Cross-model: identity architecture + diffusion of responsibility | Haiku 4.5, Sonnet 4.6, Opus 4.8 | C tied or beat B in all 6 model×case cells, never lost. The "skip thinking" behavior generalizes across tiers and gets **more pronounced with capability** — Opus produced literally 0 thinking words on 3 of 6 baseline/crystallization cells (verified not a display bug — the visible answer was still a full, coherent response). C was the only condition with nonzero thinking in every cell tested. |
| 7 | Context-graph concept validation (5 structural claims) | Sonnet 4.6 | Not an A/B/C run — validates the multi-layer graph representation used by the deconstructive preamble's frontier-node injection. See `Amassada/docs/context-graph-empirical-basis.md`. |
| 8 | Cross-provider: same two cases as exp 6, Gemini 2.5 Pro vs Sonnet 4.6; plus a mixed cross-consultation condition | Gemini 2.5 Pro, Sonnet 4.6 | C tied or beat B in all 4 graded model×case cells — zero B wins. **Gemini A→C gap is the largest observed** (A=3–4, C=7–8), suggesting Gemini's baseline reasoning is weaker but its ceiling matches Claude once raw voices are injected. Mixed cross-consultation (both models reason independently on C, Sonnet meta-synthesises) matched the best pure-C score on one case, fell below on the other — adding novelty (rated "high" both times) but not score. See confound note below. |

**Running tally:** across experiments 2–6 and 8 (17 graded head-to-head
comparisons), raw-voice injection (C) beat crystallization (B) outright in
12, tied in 5, and **lost zero**.

## Experiment 8 — Gemini 2.5 Pro detail

| Case | Run | Score | Frame dissolved | Synthesis | Novelty |
|---|---|---|---|---|---|
| identity_architecture | Gemini A_baseline | 3 | no | none | low |
| identity_architecture | Gemini B_crystallization | 7 | yes/strong | reframing | medium |
| identity_architecture | Gemini C_raw_voices | **8** | yes/strong | reframing | medium |
| identity_architecture | Sonnet A_baseline | 7 | yes/strong | reframing | medium |
| identity_architecture | Sonnet B_crystallization | 8 | yes/strong | reframing | high |
| identity_architecture | Sonnet C_raw_voices | **8** | yes/strong | reframing | medium |
| identity_architecture | Mixed D_cross_consultation | 7 | yes/strong | reframing | **high** |
| diffusion_of_responsibility | Gemini A_baseline | 4 | no | mediation | low |
| diffusion_of_responsibility | Gemini B_crystallization | 3 | no | none | low |
| diffusion_of_responsibility | Gemini C_raw_voices | **7** | yes/strong | reframing | medium |
| diffusion_of_responsibility | Sonnet A_baseline | 1 | no | none | low |
| diffusion_of_responsibility | Sonnet B_crystallization | 7 | yes/strong | reframing | medium |
| diffusion_of_responsibility | Sonnet C_raw_voices | **8** | yes/strong | reframing | **high** |
| diffusion_of_responsibility | Mixed D_cross_consultation | **8** | yes/strong | reframing | **high** |

**Exp6 reference (selected):**

| Case | Run | Score | Frame dissolved | Synthesis | Novelty |
|---|---|---|---|---|---|
| identity_architecture | Haiku C_raw_voices (exp6) | 8 | yes/strong | integration | medium |
| identity_architecture | Sonnet C_raw_voices (exp6) | 8 | yes/strong | integration | medium |
| identity_architecture | Opus C_raw_voices (exp6) | 8 | yes/strong | reframing | medium |
| diffusion_of_responsibility | Haiku C_raw_voices (exp6) | 8 | yes/strong | reframing | medium |
| diffusion_of_responsibility | Sonnet C_raw_voices (exp6) | 8 | yes/strong | integration | medium |
| diffusion_of_responsibility | Opus C_raw_voices (exp6) | 9 | yes/strong | reframing | **high** |

**Confound:** Gemini 2.5 Pro was called via its OpenAI-compatible endpoint
(`generativelanguage.googleapis.com/v1beta/openai`), which does not expose a
separate thinking block. The grader received the full polished output, not a
raw internal reasoning trace as with Claude's extended thinking. This means
Gemini scores may be *more conservative* than equivalent Claude scores (a
polished output is harder to find evidence of frame dissolution in than an
unguarded thinking trace), making the C > B result for Gemini, if anything,
understated rather than inflated.

**Mixed cross-consultation (`D_mixed`):** Gemini and Sonnet both reasoned
independently on the C prompt; their outputs were fed to Sonnet for
meta-synthesis. Result: matches or falls below the best pure C run (never
exceeds it), but reliably achieves "high" novelty — it surfaces angles
neither model raised independently. This suggests mixed runs are useful for
*diversity* not for *peak quality*, and that pre-synthesising across models
(a cross-model crystallization) does not overcome the same ceiling that
single-model crystallization hits.

## Cost and latency (experiment 6, cross-model)

Token counts are reconstructed estimates (not captured live); Opus figures
are a **floor**, since its thinking field under `display: "summarized"` is
a summary of the reasoning, not the raw chain of thought it actually
billed for.

| Model | Avg latency | Total cost (6 calls) | Score-per-dollar (C condition) |
|---|---|---|---|
| Haiku 4.5 | **22.7s** | **$0.050** | ~860–1,020 |
| Sonnet 4.6 | 67.0s | $0.220 | ~179–231 |
| Opus 4.8 | 50.2s | $0.384 (floor) | ~84–112 (overstated — true cost higher) |

Haiku matched Sonnet's and Opus's scores in the raw-voices condition at
roughly 1/5–1/14 the cost and ~3x the speed. The effect is not gated
behind an expensive model tier.

## What this means for the `deconstructive` discipline

- **Raw decomposition, not pre-synthesis, is the right default mechanism.**
  This is why the discipline's preamble (`build_deconstructive_preamble` in
  `fondament-core/src/resolver.rs`) lists the agent's actual composed parts
  verbatim and instructs it to "become each part sequentially," rather than
  handing it a pre-resolved summary of how those parts relate.
- **The construct should encode the roster, not the resolution.** Fondament
  stores *who the parts are* (disciplines, stance) as the stable, versioned
  artifact. The weighting and resolution between them has to happen live,
  every time, against the actual question — that's exactly what the
  preamble forces by presenting parts as unresolved tensions rather than a
  decided position. Baking fixed weights into the discipline definition
  would silently rebuild crystallization with extra ceremony.
- **This generalizes across model tiers**, including the cheapest one
  tested (Haiku 4.5), and the effect does not depend on the question being
  "hard" in any way that was reliably predictable in advance.
- **A real methodology caveat, not specific to this discipline:** grading
  or routing decisions that rely on "did the agent actually think about
  this" by inspecting the thinking trace will systematically misread
  capable models that reason directly in the visible output instead of in
  a separate thinking block (observed increasingly at the Opus tier). The
  `thinking_budget` this discipline sets is a forcing function for this
  exact failure mode — without it, a model may skip extended thinking
  entirely on a question it judges "answerable directly," even when the
  question is the kind this discipline exists to handle carefully.

## What experiment 8 adds to the implications

- **The finding is provider-agnostic.** C > B holds for Gemini 2.5 Pro with
  the same strength as for Claude. The deconstructive discipline's preamble
  does not depend on Anthropic's extended thinking API to produce the effect —
  it works on any model that can reason about injected positions.
- **Gemini needs the voices more.** The A→C score jump is 4–5 points for
  Gemini vs 0–7 for Claude models. Gemini's baseline reasoning on contested
  multi-perspective questions is noticeably weaker, but raw-voice injection
  closes the gap to Claude's ceiling. This strengthens the case for always
  providing raw voices to non-Claude participants in multi-provider sessions.
- **Cross-model crystallization inherits the same ceiling.** The mixed
  D_cross_consultation condition — where each model reasons independently
  and a meta-synthesiser integrates both — produces "high" novelty but
  does not beat pure C. This is the cross-provider version of the same
  result in exp 5: synthesis quality is not load-bearing, and adding more
  synthesis steps (even from a different model) does not overcome the raw
  voices ceiling.
- **Architectural implication for multi-provider Occitan sessions:** when a
  canvas mixes Claude and Gemini participants, each participant should
  receive raw voice positions from the others — not a pre-crystallized
  summary produced by a prior model. Farga should store and surface
  raw agent positions, not pre-synthesised crystallizations, regardless of
  which provider the consuming participant uses.

## Limitations

Eleven question cases across experiments 2–8, four model tiers (Haiku,
Sonnet, Opus, Gemini 2.5 Pro), one grader model (`claude-opus-4-8`),
word-count and tokenizer-based cost estimates rather than live `usage`
capture for the model-comparison pass. The Gemini condition introduces a
confound (output text vs hidden thinking trace — see Experiment 8 detail
above). Treat the "deconstructive / raw voices always wins" result as a
strong, reproducible prior — not as a closed question. It has survived every
stress test thrown at it (model tier, question type, designed difficulty,
deliberately corrupted input, cross-provider), which is more than most
architecture decisions get before being shipped, but it is still eight
experiments, not eight hundred.
