# Archive: Iteration 001

Run this prompt after completing all pre-prep integrations and the holistic pass for iteration 001. It moves all working artifacts to the archive and resets the active directory for the next iteration.

---

## Instructions

1. **Read the current iteration number** from `docs/planmaxer/.iteration`.
   Expected value: `1`

2. **Create the archive directory:**
   ```
   docs/planmaxer/archived/iteration-001/
   ```

3. **Move all contents of `docs/planmaxer/active/` into the archive:**
   ```
   docs/planmaxer/active/handoffs/    → docs/planmaxer/archived/iteration-001/handoffs/
   docs/planmaxer/active/prepreps/    → docs/planmaxer/archived/iteration-001/prepreps/
   docs/planmaxer/active/reviews/     → docs/planmaxer/archived/iteration-001/reviews/
   docs/planmaxer/active/holistic-pass.md → docs/planmaxer/archived/iteration-001/holistic-pass.md
   docs/planmaxer/active/archive.md   → docs/planmaxer/archived/iteration-001/archive.md
   ```

   Files to archive:
   - `handoffs/handoff-frontend-integration.md`
   - `handoffs/handoff-search-intelligence.md`
   - `handoffs/handoff-skill-reliability.md`
   - `handoffs/handoff-mvp-delivery.md`
   - `prepreps/preprep-frontend-integration.md`
   - `prepreps/preprep-search-intelligence.md`
   - `prepreps/preprep-skill-reliability.md`
   - `prepreps/preprep-mvp-delivery.md`
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
   Write `2` to `docs/planmaxer/.iteration`

6. **Report what was archived:**
   - Number of handoffs archived
   - Number of prepreps archived
   - Number of reviews archived (if any)
   - New iteration number: 2
   - Reminder: "Run /planmaxer to generate fresh artifacts for iteration 002"
