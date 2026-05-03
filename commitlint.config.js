// Conventional commits with crate-name scopes.
// See AGENTS.md § "Code style" for the rule.
module.exports = {
  extends: ['@commitlint/config-conventional'],
  rules: {
    'scope-enum': [
      2,
      'always',
      [
        // workspace crates
        'db',
        'durability',
        'bus',
        'lifecycle',
        'server',
        'cli',
        'planner',
        'thor',
        'executor',
        'worktree',
        'graph',
        'i18n',
        'api',
        'mcp',
        'runner',
        'tui',
        'brand',
        // meta scopes
        'docs',
        'ci',
        'chore',
        'repo',
        'deps',
        'tests',
      ],
    ],
    'scope-empty': [2, 'never'],
    'subject-case': [2, 'always', ['lower-case', 'sentence-case']],
    'header-max-length': [2, 'always', 100],
  },
};
