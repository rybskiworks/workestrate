"""Portable broker-image source contract; no service, credential or Nix calls."""

import configparser
from pathlib import Path
import re
import shlex
import unittest


HERE = Path(__file__).resolve().parent
TEMPLATE = (HERE / "workestrate-broker.service.in").read_text()
CONSTRUCTOR = (HERE / "default.nix").read_text()
SYNTHETIC_BINARY = "/nix/store/00000000000000000000000000000000-broker/bin/brokerd"
PRINCIPAL = "broker.workestrate.internal"


def render(principal=PRINCIPAL):
    """Mirror only the two literal substitutions, never interpolate unit data."""
    return TEMPLATE.replace("@brokerd@", SYNTHETIC_BINARY).replace(
        "@hostPrincipal@", principal
    )


def fields(section, text=None):
    """Retain duplicate directives such as LoadCredential in their source order."""
    active = None
    values = []
    for line in (render() if text is None else text).splitlines():
        if line.startswith("["):
            active = line
        elif active == f"[{section}]" and line and not line.startswith("#"):
            values.append(tuple(line.split("=", 1)))
    return values


class BrokerImageContract(unittest.TestCase):
    def test_template_has_only_two_literal_substitutions(self):
        self.assertEqual(re.findall(r"@[^@]+@", TEMPLATE), ["@brokerd@", "@hostPrincipal@"])
        self.assertIn('builtins.replaceStrings [ "@brokerd@" "@hostPrincipal@" ]', CONSTRUCTOR)
        self.assertIn("builtins.readFile ./workestrate-broker.service.in", CONSTRUCTOR)
        self.assertNotIn("substituteAll", CONSTRUCTOR)

    def test_service_argv_is_exact_and_ordinary(self):
        service = fields("Service")
        self.assertEqual(sum(key == "ExecStart" for key, _ in service), 1)
        self.assertEqual(
            shlex.split(dict(service)["ExecStart"]),
            [
                SYNTHETIC_BINARY,
                "service",
                "--credentials-directory",
                "%d",
                "--host-principal",
                PRINCIPAL,
                "--management-port",
                "3024",
                "--divert-port",
                "3022",
                "--egress-port",
                "3023",
            ],
        )
        self.assertEqual(dict(service)["Type"], "exec")

    def test_exact_three_runtime_credentials_without_fallback(self):
        self.assertEqual(
            [value for key, value in fields("Service") if key == "LoadCredential"],
            [
                "host-key:/broker-credentials/host-key",
                "host-certificate:/broker-credentials/host-certificate",
                "host-ca.pub:/broker-credentials/host-ca.pub",
            ],
        )
        for key, _ in fields("Service"):
            self.assertNotIn(key, {"Environment", "EnvironmentFile", "SetCredential", "LoadCredentialEncrypted"})
        self.assertNotIn("ConditionPathExists", TEMPLATE)
        self.assertNotIn("ExecCondition", TEMPLATE)
        self.assertNotIn("host-ca.key", TEMPLATE + CONSTRUCTOR)

    def test_real_registration_target_and_boot_enablement(self):
        self.assertEqual(
            fields("Unit"),
            [
                ("Description", "Workestrate managed SSH custody broker"),
                ("Requires", "guest-store-ready.target"),
                ("After", "guest-store-ready.target"),
            ],
        )
        self.assertEqual(fields("Install"), [("WantedBy", "multi-user.target")])
        self.assertIn('unitDirectory = "usr/local/lib/systemd/system";', CONSTRUCTOR)
        self.assertIn('mkdir -p "$out/${unitDirectory}/multi-user.target.wants"', CONSTRUCTOR)
        self.assertIn('ln -s ../${unitName} "$out/${unitDirectory}/multi-user.target.wants/${unitName}"', CONSTRUCTOR)

    def test_supervision_does_not_report_readiness_or_restart_implicitly(self):
        service = dict(fields("Service"))
        self.assertEqual(service["Restart"], "no")
        self.assertEqual(service["KillMode"], "control-group")
        self.assertEqual(service["TimeoutStopSec"], "30s")
        self.assertEqual(service["UMask"], "0077")
        self.assertNotIn("ExecStartPost", service)
        self.assertNotIn("NotifyAccess", service)
        self.assertNotIn("User", service)

    def test_leaf_inherits_base_without_boot_or_policy_overlay(self):
        self.assertIn("image = guest.mkNixosLayer {", CONSTRUCTOR)
        self.assertIn("inherit pkgs base;", CONSTRUCTOR)
        self.assertIn('registrationName = "broker";', CONSTRUCTOR)
        self.assertIn("contents = [ units ];", CONSTRUCTOR)
        for forbidden in (
            "mkNixosImage", "extendModules", "extraCommands", "systemd-run",
            "ExecStartPre", "SetCredential=", "nix-store", "chown",
            "/etc/systemd/system", "SYSTEMD_UNIT_PATH", "Entrypoint", "Cmd =",
        ):
            self.assertNotIn(forbidden, CONSTRUCTOR + TEMPLATE)

    def test_principal_guard_is_bounded_and_rejects_unit_injection(self):
        self.assertIn("builtins.isString hostPrincipal", CONSTRUCTOR)
        self.assertIn("builtins.stringLength hostPrincipal <= 253", CONSTRUCTOR)
        self.assertIn("builtins.stringLength label <= 63", CONSTRUCTOR)
        self.assertIn('builtins.all validLabel (lib.splitString "." hostPrincipal)', CONSTRUCTOR)
        pattern = re.search(r'builtins.match "([^"]+)" label', CONSTRUCTOR).group(1)

        def valid(value):
            return len(value) <= 253 and all(
                len(label) <= 63 and re.fullmatch(pattern, label) is not None
                for label in value.split(".")
            )

        for value in ("a", "broker.workestrate.internal", "a-b.123", "a" * 63):
            with self.subTest(value=value):
                self.assertTrue(valid(value))
        for value in (
            "", "a..b", ".a", "a.", "-a", "a-", "Broker", "a_b",
            "a" * 64, ".".join(["a" * 63] * 4), "a\nExecStart=/bin/sh",
            "a b", "%d", "$(id)", "@brokerd@", "a/b", "á",
        ):
            with self.subTest(value=value):
                self.assertFalse(valid(value))

    def test_unit_syntax_sections_and_no_unresolved_values(self):
        # ConfigParser validates section structure, while fields retains the
        # repeatable LoadCredential directives that systemd actually consumes.
        parsed = configparser.ConfigParser(interpolation=None, strict=False)
        parsed.read_string(render())
        self.assertEqual(parsed.sections(), ["Unit", "Service", "Install"])
        self.assertNotIn("@", render())
        self.assertEqual([value for key, value in fields("Service") if key == "WorkingDirectory"], ["/"])


if __name__ == "__main__":
    unittest.main()
