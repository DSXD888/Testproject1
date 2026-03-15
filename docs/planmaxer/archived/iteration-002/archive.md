# Archive: Iteration 002

Run this prompt after completing all pre-prep integrations and the holistic pass for iteration 002. It moves all working artifacts to the archive and resets the active directory for the next iteration.

---

## Instructions

1. **Read the current iteration number** from `docs/planmaxer/.iteration`.
   Expected value: `2`

2. **Create the archive directory:**
   ```
   docs/planmaxer/archived/iteration-002/
   ```

3. **Move all contents of `docs/planmaxer/active/` into the archive:**
   ```
   docs/planmaxer/active/handoffs/    → docs/planmaxer/archived/iteration-002/handoffs/
   docs/planmaxer/active/prepreps/    → docs/planmaxer/archived/iteration-002/prepreps/
   docs/planmaxer/active/reviews/     → docs/planmaxer/archived/iteration-002/reviews/
   docs/planmaxer/active/holistic-pass.md → docs/planmaxer/archived/iteration-002/holistic-pass.md
   docs/planmaxer/active/archive.md   → docs/planmaxer/archived/iteration-002/archive.md
   ```

   Files to archive:
   - `handoffs/handoff-worker-validator.md`
   - `handoffs/handoff-pipeline-integration.md`
   - `handoffs/handoff-customs-classification.md`
   - `handoffs/handoff-sync-engine.md`
   - `prepreps/preprep-worker-validator.md`
   - `prepreps/preprep-pipeline-integration.md`
   - `prepreps/preprep-customs-classification.md`
   - `prepreps/preprep-sync-engine.md`
   - `reviews/` (any review files saved by native agents)
   - `holistic-pass.md`
   - `archive.md`

4. **Recreate empty active subdirectories:**
   ```
   docs/planmaxer/active/handoffs/
   docs/planmaxer/active/prepreps/
   docs/planmaxer/active/reviews/
   ```

5. **Increment the iteration counter:**
   Write `3` to `docs/planmaxer/.iteration`

6. **Report what was archived:**
   - Number of handoffs archived
   - Number of prepreps archived
   - Number of reviews archived (if any)
   - New iteration number: 3
   - Reminder: "Run /planmaxer to generate fresh artifacts for iteration 003"
