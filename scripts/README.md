# Deployment

## Setup
```bash
npm install hardhat @nomicfoundation/hardhat-ethers ethers @openzeppelin/contracts dotenv
```

## Deploy (Local)
```bash
npx hardhat run scripts/deploy.js
```

## Deploy (Base Sepolia Testnet)
```bash
npx hardhat run scripts/deploy.js --network baseSepolia
```

## Verified Local Deployment (March 13, 2026)
| Contract | Address |
|----------|---------|
| AgentRegistry | 0x5FbDB2315678afecb367f032d93F642f64180aa3 |
| WalletFactory | 0xe7f1725E7734CE288F8367e1Bb143E90bb3F0512 |
| SimpleEscrow  | 0x9fE46736679d2D9a65F0992F2272dE9f3c7fa6e0 |
