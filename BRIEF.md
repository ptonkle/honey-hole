# Brief

Stage 1. The person's own words come first; the rest can be written by the agent from them and read by the person. `C1 Frame the problem and set the rules`, `B1 Establish context`.

## The description, word for word

Paste the description of the tool here unchanged: what you want, in your own words, as long or short as you like. A paraphrase is already an interpretation. Leave out names of people and employers if the repository will be public. Later requests in plain prose are numbered in `TRIAGE.md`.

> 

## The tool

What it does, who uses it, and on what device.

## Done means

The conditions the person will check before deploying. Make each one checkable.

- 

## Decisions that stay with the person

Content, data, anything pedagogical or specific to the person's field, any third-party asset or likeness, and anything outside `BUILDER-SPEC.md`.

- 

## Accessibility

How the tool carries information besides colour (shape, pattern, brightness, text), and the input devices it must work with.

## Style

`default`, or a path to a style pack, or a description of the look. With `default` or nothing, the art pipeline uses `styles/default/`.

## Words

Terms the person already uses in their field, with what they mean. They go into `GLOSSARY.md` before any string uses them.

## Autonomy

One paragraph: what the agent may do alone, what it must ask about, and what it may never do. State whether the plan is approved in advance and whether the agent may publish.

A paragraph that has worked for one-day builds: *The plan is approved in advance. Where this brief is silent, the agent decides, records the decision in `SECOND-ORDER.md`, and continues. The agent may not loosen a Builder requirement or a boundary test, and may not add a server, an account, analytics or any off-origin request. The agent may create a public repository named ____ and publish it with `./scripts/publish.sh` once `./scripts/check.sh` passes.*

## Allowed core dependencies

`serde`, `serde_json` (reference default). Add others only here, with the reason.
