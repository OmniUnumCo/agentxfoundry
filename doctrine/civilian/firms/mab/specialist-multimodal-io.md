---
slug: multimodal-io
kind: specialist
firm: mab
project_path: ~/Projects/multimodal-agent-builder
upstream_url: null
languages: [python, typescript]
tier_override: null
contract_scope: [text-image-audio-input, modality-consistent-typing, transcription-captioning, input-validation]
---

# CR-MAB :: Multimodal I/O

## Scope

Handling non-text inputs (image, audio, optionally video) as first-
class citizens alongside text, with consistent typing so downstream
processors don't branch per-modality. Patterns distilled from MAB's
multimodal layer.

## Core competencies

- **Unified input envelope.** A modality-agnostic wrapper (`Input
  { kind: Text | Image | Audio | Video, payload: Bytes, metadata:
  map }`) so downstream code reads one type.
- **Preprocessing pipelines.** Image resize / normalize, audio
  resample / VAD, video keyframe extraction. Deterministic where
  possible.
- **Transcription / captioning patterns.** Audio -> text via Whisper
  family; image -> text via captioning or OCR. Treating captions as
  a typed "view" of the original modality.
- **Input validation.** Size limits, format allowlists, MIME type
  sniffing, encoding detection.
- **Modality fusion.** How to combine text + image context for a
  single LLM call (prompt embedding vs separate-turn vs structured).

## When to contract

- A new agent needs image or audio input handling.
- Existing text-only pipeline needs to accept caption-of-image as
  text.
- Designing the input validation / size-limit policy for a user-
  facing agent.
- Deciding how to fuse multimodal input for an LLM.

## Out of scope

- Training multimodal models. We use off-the-shelf.
- Video analysis (object tracking, action recognition) at scale --
  keyframe+caption is the MAB pattern.
- Codec-level operations (video encoding, audio re-encoding).

## Deliverables

- `Input` envelope type adapted to the caller's target language.
- Preprocessing pipeline with explicit validation steps.
- Modality-fusion design for a specific LLM-call shape.
- Size / type-allowlist policy.

## Execution guidance

- **Prefer caption-as-text for image.** Unless the LLM natively
  takes images, captioning normalizes to text and avoids per-
  modality branching downstream.
- **Deterministic preprocessing.** Fixed seed for any probabilistic
  step (unusual in preprocessing but possible in audio VAD).
- **Explicit size limits at ingest.** Truncating or rejecting large
  inputs at the envelope layer; downstream code doesn't re-validate.
- **Metadata is first-class.** Mime type, original size, capture
  time should survive the envelope pipeline.
