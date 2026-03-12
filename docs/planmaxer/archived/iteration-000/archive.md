# Archive: Move Current Iteration to Archive

Run this prompt AFTER you have completed all handoff + pre-prep integrations AND the holistic pass for this iteration.

---

## Instructions

1. **READ** the current iteration number from `docs/planmaxer/.iteration`.
   - If the file doesn't exist or is empty, assume iteration 0.

2. **CREATE** the archive directory:
   ```
   docs/planmaxer/archived/iteration-{NNN}/
   ```
   where NNN is the current iteration number, zero-padded to 3 digits (e.g., `iteration-000`).

3. **MOVE** all contents of `docs/planmaxer/active/` into the archive directory:
   ```
   docs/planmaxer/active/handoffs/    → docs/planmaxer/archived/iteration-{NNN}/handoffs/
   docs/planmaxer/active/prepreps/    → docs/planmaxer/archived/iteration-{NNN}/prepreps/
   docs/planmaxer/active/reviews/     → docs/planmaxer/archived/iteration-{NNN}/reviews/
   docs/planmaxer/active/holistic-pass.md → docs/planmaxer/archived/iteration-{NNN}/holistic-pass.md
   docs/planmaxer/active/archive.md   → docs/planmaxer/archived/iteration-{NNN}/archive.md
   ```

4. **RECREATE** empty active/ subdirectories:
   ```
   docs/planmaxer/active/handoffs/
   docs/planmaxer/active/prepreps/
   docs/planmaxer/active/reviews/
   ```

5. **INCREMENT** the iteration counter:
   Write the number (current + 1) to `docs/planmaxer/.iteration`.

6. **REPORT** what was archived:
   - Number of handoff files archived
   - Number of pre-prep files archived
   - Number of review files archived (if any)
   - Whether holistic pass was archived
   - New iteration number
   - Reminder: "Run /planmaxer to generate fresh artifacts for iteration {N+1}"
