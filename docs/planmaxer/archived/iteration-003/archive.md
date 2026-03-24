# Archive Prompt: Iteration 003

Run this AFTER all integration and holistic passes are complete for this iteration.

---

## Instructions

1. **Read the current iteration number** from `docs/planmaxer/.iteration`
   - Expected value: `3`

2. **Create the archive directory:**
   ```
   docs/planmaxer/archived/iteration-003/
   docs/planmaxer/archived/iteration-003/handoffs/
   docs/planmaxer/archived/iteration-003/prepreps/
   docs/planmaxer/archived/iteration-003/reviews/
   ```

3. **Move all active artifacts to the archive:**
   ```
   docs/planmaxer/active/handoffs/handoff-l0-schema-foundation.md
     → docs/planmaxer/archived/iteration-003/handoffs/

   docs/planmaxer/active/handoffs/handoff-l1l2-runtime-desktop.md
     → docs/planmaxer/archived/iteration-003/handoffs/

   docs/planmaxer/active/handoffs/handoff-l3l4-validation-sync.md
     → docs/planmaxer/archived/iteration-003/handoffs/

   docs/planmaxer/active/handoffs/handoff-l5l6-commercial-coherence.md
     → docs/planmaxer/archived/iteration-003/handoffs/

   docs/planmaxer/active/prepreps/preprep-l0-schema-foundation.md
     → docs/planmaxer/archived/iteration-003/prepreps/

   docs/planmaxer/active/prepreps/preprep-l1l2-runtime-desktop.md
     → docs/planmaxer/archived/iteration-003/prepreps/

   docs/planmaxer/active/prepreps/preprep-l3l4-validation-sync.md
     → docs/planmaxer/archived/iteration-003/prepreps/

   docs/planmaxer/active/prepreps/preprep-l5l6-commercial-coherence.md
     → docs/planmaxer/archived/iteration-003/prepreps/

   docs/planmaxer/active/holistic-pass.md
     → docs/planmaxer/archived/iteration-003/

   docs/planmaxer/active/archive.md
     → docs/planmaxer/archived/iteration-003/
   ```

   **Also move any review files** from `docs/planmaxer/active/reviews/` if they exist.

   **Also move the PREVIOUS iteration 003 artifacts** (from the earlier generation targeting the research doc) if they still exist in active/:
   ```
   docs/planmaxer/active/handoffs/handoff-worker-validator.md (if exists)
   docs/planmaxer/active/handoffs/handoff-sync-engine.md (if exists)
   docs/planmaxer/active/handoffs/handoff-pipeline-integration.md (if exists)
   docs/planmaxer/active/handoffs/handoff-customs-classification.md (if exists)
   docs/planmaxer/active/prepreps/preprep-worker-validator.md (if exists)
   docs/planmaxer/active/prepreps/preprep-sync-engine.md (if exists)
   docs/planmaxer/active/prepreps/preprep-pipeline-integration.md (if exists)
   docs/planmaxer/active/prepreps/preprep-customs-classification.md (if exists)
   ```
   Move these to the same archive directory.

4. **Recreate empty active subdirectories:**
   ```
   docs/planmaxer/active/handoffs/
   docs/planmaxer/active/prepreps/
   docs/planmaxer/active/reviews/
   ```

5. **Increment the iteration counter:**
   Write `4` to `docs/planmaxer/.iteration`

6. **Report:**
   - Artifacts archived: 4 handoffs (strategy-layer-focused) + 4 prepreps + holistic pass + archive prompt
   - Also archived: up to 4 prior handoffs + 4 prior prepreps (research-doc-focused, from earlier generation)
   - New iteration: 4
   - Reminder: Run `/planmaxer` to generate fresh artifacts for iteration 004
