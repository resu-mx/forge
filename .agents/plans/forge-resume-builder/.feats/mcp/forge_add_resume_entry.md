Filler Bullets — Problem

These 4 bullets don't have perspectives yet. The forge_add_resume_entry requires a perspective_id, and perspectives are created through the derivation flow (prepare → commit). The bullet text is already resume-ready, so we need to
either:

Option A: Run derivation (prepare/commit) for each bullet to create perspectives — then add to resume
Option B: Create perspectives directly via the API/DB, using the bullet text as-is

Which do you prefer? Option B is faster since the text doesn't need reframing for filler.
