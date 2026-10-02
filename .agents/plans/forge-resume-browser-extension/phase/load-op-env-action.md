# load-secrets-action Fork: 1Password Environments Support

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Fork `1Password/load-secrets-action`, add support for reading secrets from 1Password Environments via the JS SDK, publish as `aRustyDev/load-secrets-action`.

**Architecture:** Additive change to the existing action. When `environment-id` input is provided, use `@1password/sdk` to call `client.environments.getVariables()` instead of the `op` CLI path. Existing `op://` reference resolution remains untouched.

**Tech Stack:** TypeScript, `@1password/sdk@0.4.1-beta.1`, `@actions/core`, `@vercel/ncc`, Jest

---

## File Map

### In `aRustyDev/load-secrets-action` (forked repo)

| File | Action | Purpose |
|------|--------|---------|
| `action.yml` | Modify | Add `environment-id` input |
| `package.json` | Modify | Add `@1password/sdk@0.4.1-beta.1` dependency |
| `src/constants.ts` | Modify | Add `envEnvironmentId` constant |
| `src/environments.ts` | Create | `loadEnvironmentSecrets()` — SDK-based environment loading |
| `src/environments.test.ts` | Create | Tests for environment loading |
| `src/index.ts` | Modify | Branch on `environment-id` input |
| `src/__mocks__/op-sdk.ts` | Create | Mock for `@1password/sdk` |
| `config/jest.config.js` | Modify | Add SDK mock mapping |
| `dist/index.js` | Rebuild | `npm run build` after changes |

---

## Task 1: Fork and clone

- [ ] **Step 1: Fork the repo**

```bash
gh repo fork 1Password/load-secrets-action --clone=false --fork-name load-secrets-action
```

- [ ] **Step 2: Clone via SSH**

```bash
git clone git@github.com:aRustyDev/load-secrets-action.git ~/code/proj/load-secrets-action
cd ~/code/proj/load-secrets-action
```

- [ ] **Step 3: Install dependencies**

```bash
npm install
```

- [ ] **Step 4: Run existing tests to verify baseline**

```bash
npm test
```

Expected: All tests pass.

- [ ] **Step 5: Create feature branch**

```bash
git checkout -b feat/environments-support
```

---

## Task 2: Add `environment-id` input to action.yml

**Files:**
- Modify: `action.yml`

- [ ] **Step 1: Add the new input**

Add after the `version` input:

```yaml
  environment-id:
    description: "1Password Environment ID. When set, loads all variables from the specified Environment using the 1Password SDK instead of op:// secret references."
    required: false
```

- [ ] **Step 2: Commit**

```bash
git add action.yml
git commit -m "feat: add environment-id input to action.yml"
```

---

## Task 3: Add SDK dependency and constants

**Files:**
- Modify: `package.json`
- Modify: `src/constants.ts`

- [ ] **Step 1: Install the SDK**

```bash
npm install @1password/sdk@0.4.1-beta.1
```

- [ ] **Step 2: Add environment constant**

In `src/constants.ts`, add:

```typescript
export const envEnvironmentId = "INPUT_ENVIRONMENT-ID";
```

Note: GitHub Actions converts input names to env vars as `INPUT_<UPPERCASED-NAME>` with hyphens preserved. But `@actions/core.getInput()` handles the lookup — we'll use that instead of the raw env var. The constant is for documentation/reference.

- [ ] **Step 3: Commit**

```bash
git add package.json package-lock.json src/constants.ts
git commit -m "feat: add @1password/sdk dependency and environment-id constant"
```

---

## Task 4: Create environments.ts — SDK-based environment loading

**Files:**
- Create: `src/environments.ts`

- [ ] **Step 1: Write the environment loading module**

Create `src/environments.ts`:

```typescript
import * as core from "@actions/core";
import { createClient } from "@1password/sdk";
import { envManagedVariables, envServiceAccountToken } from "./constants";

/**
 * Load all variables from a 1Password Environment using the JS SDK.
 *
 * This is an alternative to the op CLI path used by loadSecrets().
 * When `environment-id` input is provided, this function is called
 * instead of the CLI-based flow.
 */
export const loadEnvironmentSecrets = async (
	environmentId: string,
	shouldExportEnv: boolean,
): Promise<void> => {
	const token = process.env[envServiceAccountToken];
	if (!token) {
		throw new Error(
			`${envServiceAccountToken} is required when using environment-id.`,
		);
	}

	core.info(`Loading secrets from 1Password Environment: ${environmentId}`);

	const client = await createClient({
		auth: token,
		integrationName: "1Password GitHub Action",
		integrationVersion: "v1.0.0",
	});

	const response = await client.environments.getVariables(environmentId);

	if (!response.variables || response.variables.length === 0) {
		core.warning(
			`No variables found in 1Password Environment: ${environmentId}`,
		);
		return;
	}

	const managedEnvNames: string[] = [];

	for (const { name, value } of response.variables) {
		core.info(`Populating variable: ${name}`);

		// Mask non-empty values in logs
		if (value) {
			core.setSecret(value);
		}

		if (shouldExportEnv) {
			core.exportVariable(name, value);
		} else {
			core.setOutput(name, value);
		}

		managedEnvNames.push(name);
	}

	if (shouldExportEnv) {
		core.exportVariable(envManagedVariables, managedEnvNames.join());
	}

	core.info(
		`Loaded ${response.variables.length} variables from 1Password Environment.`,
	);
};
```

- [ ] **Step 2: Commit**

```bash
git add src/environments.ts
git commit -m "feat: add loadEnvironmentSecrets() using 1Password SDK"
```

---

## Task 5: Wire environments into index.ts

**Files:**
- Modify: `src/index.ts`

- [ ] **Step 1: Add the environment branch to the main flow**

Add the import at the top:

```typescript
import { loadEnvironmentSecrets } from "./environments";
```

In the `loadSecretsAction` function, after the `validateAuth()` call and before the `installCLI()` call, add the environment-id branch:

```typescript
		// Check if environment-id is provided — use SDK path instead of CLI
		const environmentId = core.getInput("environment-id");
		if (environmentId) {
			await loadEnvironmentSecrets(environmentId, shouldExportEnv);
			return;
		}
```

This goes after `validateAuth()` so auth is still verified, but before `installCLI()` and the dotenv/`loadSecrets()` calls — those are skipped entirely when using environments.

- [ ] **Step 2: Commit**

```bash
git add src/index.ts
git commit -m "feat: branch on environment-id input in main action flow"
```

---

## Task 6: Add tests for environments

**Files:**
- Create: `src/__mocks__/op-sdk.ts`
- Create: `src/environments.test.ts`
- Modify: `config/jest.config.js`

- [ ] **Step 1: Create the SDK mock**

Create `src/__mocks__/op-sdk.ts`:

```typescript
const mockGetVariables = jest.fn(() =>
	Promise.resolve({
		variables: [
			{ name: "DB_HOST", value: "localhost", masked: false },
			{ name: "DB_PASSWORD", value: "s3cret", masked: true },
		],
	}),
);

const mockCreateClient = jest.fn(() =>
	Promise.resolve({
		environments: {
			getVariables: mockGetVariables,
		},
	}),
);

module.exports = {
	createClient: mockCreateClient,
	__mockGetVariables: mockGetVariables,
	__mockCreateClient: mockCreateClient,
};
```

- [ ] **Step 2: Add mock mapping to jest config**

In `config/jest.config.js`, add to `moduleNameMapper`:

```javascript
		"^@1password/sdk$": "<rootDir>/__mocks__/op-sdk.ts",
```

- [ ] **Step 3: Write the tests**

Create `src/environments.test.ts`:

```typescript
import * as core from "@actions/core";
import { loadEnvironmentSecrets } from "./environments";
import { envManagedVariables, envServiceAccountToken } from "./constants";

jest.mock("@1password/sdk");

// eslint-disable-next-line @typescript-eslint/no-require-imports
const { __mockGetVariables, __mockCreateClient } = require("@1password/sdk");

beforeEach(() => {
	jest.clearAllMocks();
	process.env[envServiceAccountToken] = "ops_test_token";
});

afterEach(() => {
	delete process.env[envServiceAccountToken];
});

describe("loadEnvironmentSecrets", () => {
	const testEnvironmentId = "env_abc123";

	it("should throw if OP_SERVICE_ACCOUNT_TOKEN is not set", async () => {
		delete process.env[envServiceAccountToken];
		await expect(
			loadEnvironmentSecrets(testEnvironmentId, true),
		).rejects.toThrow("OP_SERVICE_ACCOUNT_TOKEN is required");
	});

	it("should create client with service account token", async () => {
		await loadEnvironmentSecrets(testEnvironmentId, true);
		expect(__mockCreateClient).toHaveBeenCalledWith({
			auth: "ops_test_token",
			integrationName: "1Password GitHub Action",
			integrationVersion: "v1.0.0",
		});
	});

	it("should call getVariables with the environment ID", async () => {
		await loadEnvironmentSecrets(testEnvironmentId, true);
		expect(__mockGetVariables).toHaveBeenCalledWith(testEnvironmentId);
	});

	it("should export variables as env vars when shouldExportEnv is true", async () => {
		await loadEnvironmentSecrets(testEnvironmentId, true);
		expect(core.exportVariable).toHaveBeenCalledWith("DB_HOST", "localhost");
		expect(core.exportVariable).toHaveBeenCalledWith("DB_PASSWORD", "s3cret");
		expect(core.setOutput).not.toHaveBeenCalledWith(
			"DB_HOST",
			expect.anything(),
		);
	});

	it("should set variables as outputs when shouldExportEnv is false", async () => {
		await loadEnvironmentSecrets(testEnvironmentId, false);
		expect(core.setOutput).toHaveBeenCalledWith("DB_HOST", "localhost");
		expect(core.setOutput).toHaveBeenCalledWith("DB_PASSWORD", "s3cret");
		expect(core.exportVariable).not.toHaveBeenCalled();
	});

	it("should mask non-empty secret values", async () => {
		await loadEnvironmentSecrets(testEnvironmentId, true);
		expect(core.setSecret).toHaveBeenCalledWith("localhost");
		expect(core.setSecret).toHaveBeenCalledWith("s3cret");
	});

	it("should not call setSecret for empty values", async () => {
		__mockGetVariables.mockResolvedValueOnce({
			variables: [{ name: "EMPTY", value: "", masked: false }],
		});
		await loadEnvironmentSecrets(testEnvironmentId, true);
		expect(core.setSecret).not.toHaveBeenCalled();
	});

	it("should track managed variables when exporting env", async () => {
		await loadEnvironmentSecrets(testEnvironmentId, true);
		expect(core.exportVariable).toHaveBeenCalledWith(
			envManagedVariables,
			"DB_HOST,DB_PASSWORD",
		);
	});

	it("should warn when no variables found", async () => {
		__mockGetVariables.mockResolvedValueOnce({ variables: [] });
		await loadEnvironmentSecrets(testEnvironmentId, true);
		expect(core.warning).toHaveBeenCalledWith(
			expect.stringContaining("No variables found"),
		);
	});

	it("should log the count of loaded variables", async () => {
		await loadEnvironmentSecrets(testEnvironmentId, true);
		expect(core.info).toHaveBeenCalledWith(
			"Loaded 2 variables from 1Password Environment.",
		);
	});
});
```

- [ ] **Step 4: Run tests**

```bash
npm test
```

Expected: All existing tests pass + 9 new environment tests pass.

- [ ] **Step 5: Commit**

```bash
git add src/__mocks__/op-sdk.ts src/environments.test.ts config/jest.config.js
git commit -m "test: add tests for loadEnvironmentSecrets"
```

---

## Task 7: Build dist and push

**Files:**
- Rebuild: `dist/index.js`

- [ ] **Step 1: Build the bundled dist**

```bash
npm run build
```

Expected: `dist/index.js` regenerated with SDK code bundled.

Note: The `@1password/sdk` package includes a WASM binary. Verify the build output works — `ncc` should inline it. If the WASM file isn't bundled, check if `ncc` needs `--asset` flags or if the SDK expects a filesystem path for the WASM. If this is a problem, we may need to adjust the build configuration.

- [ ] **Step 2: Verify dist size is reasonable**

```bash
ls -lh dist/index.js
```

The original dist is likely ~500KB-1MB. With the SDK it may be larger due to WASM.

- [ ] **Step 3: Run format and lint checks**

```bash
npm run format:check || npm run format:write
npm run lint || npm run lint:fix
```

Fix any issues.

- [ ] **Step 4: Commit dist**

```bash
git add dist/
git commit -m "chore: rebuild dist with environments support"
```

- [ ] **Step 5: Push the feature branch**

```bash
git push -u origin feat/environments-support
```

- [ ] **Step 6: Merge to main**

```bash
git checkout main
git merge feat/environments-support
git push
```

---

## Task 8: Update forge workflow to use the fork

**Repo:** `aRustyDev/forge`

**Files:**
- Delete: `.github/actions/load-op-env/` (the composite action we're replacing)
- Modify: `.github/workflows/extension-publish.yml`

- [ ] **Step 1: Remove the old composite action**

```bash
rm -rf .github/actions/load-op-env
```

- [ ] **Step 2: Update workflow to use the fork**

Replace the `load-op-env` action references with:

```yaml
      - name: Load 1Password secrets
        uses: aRustyDev/load-secrets-action@main
        with:
          environment-id: ${{ secrets.OP_ENVIRONMENT_ID }}
          export-env: true
        env:
          OP_SERVICE_ACCOUNT_TOKEN: ${{ secrets.OP_SVC_ACCT_TOKEN }}
```

Note the auth pattern: `OP_SERVICE_ACCOUNT_TOKEN` is passed as an env var (matching the original action's auth pattern), not as an input. The `environment-id` is the new input.

- [ ] **Step 3: Commit and push**

```bash
git add -A
git commit -m "feat: switch to aRustyDev/load-secrets-action fork with environments support"
git push
```

---

## Summary

| Task | Repo | Description |
|------|------|-------------|
| 1 | load-secrets-action | Fork, clone, verify baseline |
| 2 | load-secrets-action | Add `environment-id` input |
| 3 | load-secrets-action | Add SDK dependency + constant |
| 4 | load-secrets-action | Create `environments.ts` |
| 5 | load-secrets-action | Wire into `index.ts` |
| 6 | load-secrets-action | Tests + mocks |
| 7 | load-secrets-action | Build dist, push |
| 8 | forge | Update workflow to use fork |
