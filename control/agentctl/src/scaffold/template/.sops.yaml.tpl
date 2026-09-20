# SOPS config for {{ config_name }} fleet.
# Age recipient: personal key.
# Generate your age key:
#   age-keygen -o ~/.config/sops/age/ai-workbench-secrets.txt
# Get your public key:
#   age-keygen -y ~/.config/sops/age/ai-workbench-secrets.txt

keys:
  - &{{ config_name }} {{ age_recipient }}
creation_rules:
  # Personal-only secrets: encrypted to personal key.
  - path_regex: ^secrets/shared/.*\.enc$
    key_groups:
      - age:
          - *{{ config_name }}
  - path_regex: ^secrets/{{ config_name }}/.*\.enc$
    key_groups:
      - age:
          - *{{ config_name }}
  - path_regex: ^\.env\.enc$
    key_groups:
      - age:
          - *{{ config_name }}
