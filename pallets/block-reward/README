README
# peaq-block-reward-pallet

> Block reward pallet is for configuring the reward distribution, setting up reward per block and hard capping reward generation.

## Overview

To call these extrinsic go to the Polkadot app and switch to agung network.

Go to **Developer → Extrinsics**. And choose the `blockReward` pallet from the list.

Reward pallet has 3 extrinsic calls as of now.

- `setBlockIssueReward`

Params - blockReward.

Description - For setting up the reward amount to generate after each block.

- `setConfiguration`

Params - **rewardDistroParams** -> 
* treasuryPercent 
* dappsPercent 
* collatorsPercent 
* lpPercent 
* machinesPercent 
* machinesSubsidizationPercent.

Description - For configuring reward distribution between these accounts mentioned above. Each account gets a percentage of block reward.


- `setHardCap`

Params - limit.

Description - For setting up hard cap on block reward. After the total token issuance have reached the hard cap limit then block reward generation will stop.
