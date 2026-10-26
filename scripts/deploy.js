const hre = require("hardhat");

async function main() {
  const [deployer] = await hre.ethers.getSigners();
  console.log("Deploying with:", deployer.address);
  
  const balance = await hre.ethers.provider.getBalance(deployer.address);
  console.log("Network:", hre.network.name, "chainId", (await hre.ethers.provider.getNetwork()).chainId.toString());
  console.log("Balance:", hre.ethers.formatEther(balance), "(native)\n");

  // 1. Deploy AgentRegistry
  console.log("Deploying AgentRegistry...");
  const AgentRegistry = await hre.ethers.getContractFactory("AgentRegistry");
  const registry = await AgentRegistry.deploy();
  await registry.waitForDeployment();
  const registryAddr = await registry.getAddress();
  console.log("✅ AgentRegistry:", registryAddr);

  // 2. Deploy WalletFactory
  console.log("Deploying WalletFactory...");
  const WalletFactory = await hre.ethers.getContractFactory("WalletFactory");
  const factory = await WalletFactory.deploy(registryAddr);
  await factory.waitForDeployment();
  const factoryAddr = await factory.getAddress();
  console.log("✅ WalletFactory:", factoryAddr);

  // 3. Deploy SimpleEscrow
  console.log("Deploying SimpleEscrow...");
  const SimpleEscrow = await hre.ethers.getContractFactory("SimpleEscrow");
  const escrow = await SimpleEscrow.deploy();
  await escrow.waitForDeployment();
  const escrowAddr = await escrow.getAddress();
  console.log("✅ SimpleEscrow:", escrowAddr);

  console.log(`\n🚀 AXON Protocol — ${hre.network.name}`);
  console.log("AgentRegistry:", registryAddr);
  console.log("WalletFactory:", factoryAddr);
  console.log("SimpleEscrow: ", escrowAddr);
}

main().catch((error) => {
  console.error(error);
  process.exitCode = 1;
});
