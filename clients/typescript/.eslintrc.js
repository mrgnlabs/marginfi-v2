module.exports = {
  root: true,
  // This tells ESLint to load the config from the package `eslint-config-custom`
  extends: ["eslint-config-mrgnlabs"],
  settings: {
    next: {
      rootDir: ["apps/*/"],
    },
  },
};
