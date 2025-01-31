const anchor = require('@project-serum/anchor');

module.exports = async function(provider) {
  anchor.setProvider(provider);
  const program = anchor.workspace.SurveytrendToken;
  console.log("Deploying token program:", program.programId.toString());
};
