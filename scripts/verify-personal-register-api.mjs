import assert from "node:assert/strict";
import path from "node:path";
import { mkdir, writeFile } from "node:fs/promises";
import {
  nextPort,
  requestJson,
  requestJsonAllowError,
  startTempAdminApi,
} from "./helpers/temp-admin-api.mjs";

function nowSuffix() {
  return new Date().toISOString().replace(/[-:.TZ]/g, "").slice(0, 14);
}

async function main() {
  const port = Number(process.env.SSH_ASSISTANT_REGISTER_TEST_PORT || nextPort(5080));
  const server = await startTempAdminApi({
    port,
    label: "register-api",
  });

  const suffix = nowSuffix();
  const email = `register-${suffix}@example.com`;
  const displayName = `Register ${suffix}`;
  const password = "secret123";
  const duplicateAdminId = `usr-admin-dup-${suffix}`;

  const findings = [];
  const record = (id, status, detail, extra = {}) => {
    findings.push({ id, status, detail, ...extra });
  };

  try {
    const registerResponse = await requestJson(server.baseUrl, "/api/client/register", {
      method: "POST",
      body: JSON.stringify({
        email,
        displayName,
        password,
      }),
    });

    assert.equal(registerResponse.mode, "personal");
    assert.equal(registerResponse.email, email);
    assert.equal(registerResponse.displayName, displayName);
    assert.equal(registerResponse.aiSubscription.planName, "free");
    assert(
      String(registerResponse.aiSubscription.status).toLowerCase() === "1" ||
        String(registerResponse.aiSubscription.status).toLowerCase() === "trialing",
      "Register response should default to trialing status.",
    );
    assert.equal(
      String(registerResponse.subscriptionSnapshot?.subscription?.billingScope || "").toLowerCase(),
      "personal",
    );
    record("register-success", "passed", "Public personal registration returned personal login state.", {
      email,
      displayName,
      planName: registerResponse.aiSubscription.planName,
      subscriptionStatus: registerResponse.aiSubscription.status,
      accountKey: registerResponse.accountKey,
    });

    const duplicateRegister = await requestJsonAllowError(server.baseUrl, "/api/client/register", {
      method: "POST",
      body: JSON.stringify({
        email,
        displayName,
        password,
      }),
    });
    assert.equal(duplicateRegister.status, 409);
    assert.equal(
      duplicateRegister.payload?.error,
      "This email is already registered. Please sign in instead.",
    );
    record("register-duplicate", "passed", "Duplicate public registration is rejected with 409 conflict.");

    const loginResponse = await requestJson(server.baseUrl, "/api/client/login", {
      method: "POST",
      body: JSON.stringify({
        mode: "personal",
        identifier: email,
        secret: password,
      }),
    });
    assert.equal(loginResponse.mode, "personal");
    assert.equal(loginResponse.email, email);
    assert.equal(loginResponse.displayName, displayName);
    record("register-login", "passed", "Newly registered personal account can log in again with email and password.");

    const runtimeResponse = await requestJson(
      server.baseUrl,
      `/api/client/ai/runtime?${new URLSearchParams({
        accessToken: registerResponse.accessToken,
      })}`,
    );
    assert.equal(runtimeResponse.enabled, true);
    record("register-trialing-runtime", "passed", "Trialing personal registration can use the managed AI runtime.");

    const adminLogin = await requestJson(server.baseUrl, "/api/admin/login", {
      method: "POST",
      body: JSON.stringify({
        username: "admin",
        password: "admin123",
      }),
    });
    const adminHeaders = {
      Authorization: `Bearer ${adminLogin.token}`,
    };

    const adminDuplicate = await requestJsonAllowError(server.baseUrl, "/api/admin/personal-accounts", {
      method: "POST",
      headers: adminHeaders,
      body: JSON.stringify({
        id: duplicateAdminId,
        displayName: "Duplicate Admin Account",
        email,
        secret: "admin-secret-123",
        subscriptionStatus: 0,
        planName: "free",
        customEndpointEnabled: true,
      }),
    });
    assert.equal(adminDuplicate.status, 409);
    assert.equal(
      adminDuplicate.payload?.error,
      "This email is already registered. Please sign in instead.",
    );
    record("admin-duplicate", "passed", "Admin personal account upsert also enforces email uniqueness.");

    const adminCreatedEmail = `admin-created-${suffix}@example.com`;
    const adminCreatedPassword = "admin-created-secret";
    const adminCreatedAccount = await requestJson(server.baseUrl, "/api/admin/personal-accounts", {
      method: "POST",
      headers: adminHeaders,
      body: JSON.stringify({
        id: "",
        displayName: `Admin Created ${suffix}`,
        email: adminCreatedEmail,
        secret: adminCreatedPassword,
        subscriptionStatus: 1,
        planName: "free",
        customEndpointEnabled: true,
      }),
    });
    assert.equal(adminCreatedAccount.email, adminCreatedEmail);

    const adminCreatedLogin = await requestJson(server.baseUrl, "/api/client/login", {
      method: "POST",
      body: JSON.stringify({
        mode: "personal",
        identifier: adminCreatedEmail,
        secret: adminCreatedPassword,
      }),
    });
    assert.equal(adminCreatedLogin.email, adminCreatedEmail);
    record("admin-create-login", "passed", "Admin-created personal account remains loginable by email.");

    const adminEditedPassword = "admin-edited-secret";
    const adminEditedDisplayName = `Admin Edited ${suffix}`;
    const adminEditedAccount = await requestJson(server.baseUrl, "/api/admin/personal-accounts", {
      method: "POST",
      headers: adminHeaders,
      body: JSON.stringify({
        id: adminCreatedAccount.id,
        displayName: adminEditedDisplayName,
        email: adminCreatedEmail,
        secret: adminEditedPassword,
        subscriptionStatus: 1,
        planName: "free",
        customEndpointEnabled: true,
      }),
    });
    assert.equal(adminEditedAccount.id, adminCreatedAccount.id);
    assert.equal(adminEditedAccount.displayName, adminEditedDisplayName);

    const adminEditedLogin = await requestJson(server.baseUrl, "/api/client/login", {
      method: "POST",
      body: JSON.stringify({
        mode: "personal",
        identifier: adminCreatedEmail,
        secret: adminEditedPassword,
      }),
    });
    assert.equal(adminEditedLogin.email, adminCreatedEmail);
    assert.equal(adminEditedLogin.displayName, adminEditedDisplayName);
    record("admin-edit-login", "passed", "Admin-edited personal account remains loginable by email and updated secret.");

    const concurrentEmail = `concurrent-${suffix}@example.com`;
    const concurrentRequestBody = JSON.stringify({
      email: concurrentEmail,
      displayName: `Concurrent ${suffix}`,
      password,
    });

    const [concurrentA, concurrentB] = await Promise.all([
      requestJsonAllowError(server.baseUrl, "/api/client/register", {
        method: "POST",
        body: concurrentRequestBody,
      }),
      requestJsonAllowError(server.baseUrl, "/api/client/register", {
        method: "POST",
        body: concurrentRequestBody,
      }),
    ]);

    const statuses = [concurrentA.status, concurrentB.status].sort((a, b) => a - b);
    assert.deepEqual(statuses, [200, 409]);
    record(
      "register-concurrent",
      "passed",
      "Concurrent duplicate registration results in exactly one success and one conflict.",
      {
        statuses,
      },
    );

    const output = {
      ok: true,
      baseUrl: server.baseUrl,
      findings,
      evidence: {
        registeredEmail: email,
        concurrentEmail,
      },
    };

    const outputDir = path.resolve("tmp", "personal-register");
    await mkdir(outputDir, { recursive: true });
    const jsonPath = path.join(outputDir, "verify-personal-register-api.json");
    await writeFile(jsonPath, `${JSON.stringify(output, null, 2)}\n`, "utf8");
    console.log(
      JSON.stringify(
        {
          ...output,
          artifacts: {
            jsonPath,
            tempDir: server.tempDir,
          },
        },
        null,
        2,
      ),
    );
  } finally {
    await server.stop();
  }
}

main().catch((error) => {
  console.error(
    JSON.stringify(
      {
        ok: false,
        error: String(error?.message || error),
      },
      null,
      2,
    ),
  );
  process.exitCode = 1;
});
