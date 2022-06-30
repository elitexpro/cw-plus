#!/bin/bash

#Build Flag
PARAM=$1
SUBPARAM=$2
####################################    Constants    ##################################################

#depends on mainnet or testnet
NODE="--node https://rpc-juno.itastakers.com:443"
CHAIN_ID=juno-1
DENOM="ujuno"

##########################################################################################

# NODE="--node https://rpc.juno.giansalex.dev:443"
# CHAIN_ID=uni-3
# DENOM="ujunox"

##########################################################################################
#not depends
NODECHAIN=" $NODE --chain-id $CHAIN_ID"
TXFLAG=" $NODECHAIN --gas-prices 0.001$DENOM --gas auto --gas-adjustment 1.3"
WALLET="--from new_marble"

RELEASE="release/"

INFO="mainnet/"
CW721WASMFILE=$RELEASE"cw721_base.wasm"
PRESALEAIRDROPWASMFILE=$RELEASE"marble_presaleairdrop.wasm"
COLLECTIONWASMFILE=$RELEASE"marble_collection.wasm"
MARKETPLACEWASMFILE=$RELEASE"marble_marketplace.wasm"

FILE_CODE_CW721_BASE=$INFO"code_cw721_base.txt"
FILE_CODE_MARBLE_COLLECTION=$INFO"code_marble_collection.txt"
FILE_CODE_MARBLE_MARKETPLACE=$INFO"code_marble_marketplace.txt"
FILE_CODE_MARBLE_PRESALEAIRDROP=$INFO"code_marble_presaleairdrop.txt"

FILE_UPLOADHASH=$INFO"uploadtx.txt"
FILE_PRESALE_CONTRACT_ADDR=$INFO"contract_presale.txt"
FILE_AIRDROP_CONTRACT_ADDR=$INFO"contract_airdrop.txt"
FILE_MARKETPLACE_CONTRACT_ADDR=$INFO"contract_marketplace.txt"

ADDR_ACHILLES="juno15fg4zvl8xgj3txslr56ztnyspf3jc7n9j44vhz"
ADDR_MARBLE="juno1zzru8wptsc23z2lw9rvw4dq606p8fz0z6k6ggn"

# TOKEN_MARBLE="juno15s50e6k9s8mac9cmrg2uq85cgw7fxxfh24xhr0chems2rjxsfjjs8kmuje"

TOKEN_MARBLE="juno1g2g7ucurum66d42g8k5twk34yegdq8c82858gz0tq2fc75zy7khssgnhjl"

AIRDROP_LIST_1="airdroplist/earlylp.json"
AIRDROP_LIST_2="airdroplist/final-daodao.json"
FILE_MERKLEROOT="merkleroot.txt"
###################################################################################################
###################################################################################################
###################################################################################################
###################################################################################################
#Environment Functions
CreateEnv() {
    sudo apt-get update && sudo apt upgrade -y
    sudo apt-get install make build-essential gcc git jq chrony -y
    wget https://golang.org/dl/go1.17.3.linux-amd64.tar.gz
    sudo tar -C /usr/local -xzf go1.17.3.linux-amd64.tar.gz
    rm -rf go1.17.3.linux-amd64.tar.gz

    export GOROOT=/usr/local/go
    export GOPATH=$HOME/go
    export GO111MODULE=on
    export PATH=$PATH:/usr/local/go/bin:$HOME/go/bin
    
    rustup default stable
    rustup target add wasm32-unknown-unknown

    git clone https://github.com/CosmosContracts/juno
    cd juno
    git fetch
    git checkout v6.0.0
    make install
    cd ../
    rm -rf juno

    junod keys import new_marble new_marble.key
}

RustBuild() {

    echo "================================================="
    echo "Rust Optimize Build Start"
    
    rm -rf release
    mkdir release
    
    cd contracts
    
    cd cw721-base
    RUSTFLAGS='-C link-arg=-s' cargo wasm
    cp target/wasm32-unknown-unknown/release/*.wasm ../../release/

    cd ..
    cd collection
    RUSTFLAGS='-C link-arg=-s' cargo wasm
    cp target/wasm32-unknown-unknown/release/*.wasm ../../release/

    cd ..
    cd presaleairdrop
    RUSTFLAGS='-C link-arg=-s' cargo wasm
    cp target/wasm32-unknown-unknown/release/*.wasm ../../release/

    cd ..
    cd marketplace
    RUSTFLAGS='-C link-arg=-s' cargo wasm
    cp target/wasm32-unknown-unknown/release/*.wasm ../../release/
}

Upload() {
    echo "================================================="
    echo "Upload $SUBPARAM"
    
    UPLOADTX=$(junod tx wasm store $RELEASE$SUBPARAM".wasm" $WALLET $TXFLAG --output json -y | jq -r '.txhash')
    
    echo "Upload txHash:"$UPLOADTX
    
    echo "================================================="
    echo "GetCode"
	CODE_ID=""
    while [[ $CODE_ID == "" ]]
    do 
        sleep 3
        CODE_ID=$(junod query tx $UPLOADTX $NODECHAIN --output json | jq -r '.logs[0].events[-1].attributes[0].value')
    done
    echo "Contract Code_id:"$CODE_ID

    #save to FILE_CODE_ID
    echo $CODE_ID > $INFO"code_"$SUBPARAM".txt"
}

InstantiatePresale() { 
    echo "================================================="
    echo "Instantiate Presale Contract"
    CODE_MARBLE_PRESALEAIRDROP=$(cat $FILE_CODE_MARBLE_PRESALEAIRDROP)
    CODE_CW721_BASE=$(cat $FILE_CODE_CW721_BASE)

    echo "PresaleAirdrop Code ID: "$CODE_MARBLE_PRESALEAIRDROP
    echo "CW721-base Code ID: "$CODE_CW721_BASE
    
    TXHASH=$(junod tx wasm instantiate $CODE_MARBLE_PRESALEAIRDROP '{"owner":"'$ADDR_MARBLE'", "pay_native": true, "airdrop": false, "native_denom":"'$DENOM'", "max_tokens":100000, "name":"MarbleNFT", "symbol":"MNFT", "token_code_id": '$CODE_CW721_BASE', "cw20_address":"'$TOKEN_MARBLE'", "royalty":0}' --admin $ADDR_MARBLE --label "Marblenauts" $WALLET $TXFLAG -y --output json | jq -r '.txhash')
    echo $TXHASH
    CONTRACT_ADDR=""
    while [[ $CONTRACT_ADDR == "" ]]
    do
        sleep 3
        CONTRACT_ADDR=$(junod query tx $TXHASH $NODECHAIN --output json | jq -r '.logs[0].events[0].attributes[0].value')
    done
    echo $CONTRACT_ADDR
    echo $CONTRACT_ADDR > $FILE_PRESALE_CONTRACT_ADDR
}

InstantiateAirdrop() { 
    echo "================================================="
    echo "Instantiate Airdrop Contract"
    CODE_MARBLE_PRESALEAIRDROP=$(cat $FILE_CODE_MARBLE_PRESALEAIRDROP)
    CODE_CW721_BASE=$(cat $FILE_CODE_CW721_BASE)

    echo "PresaleAirdrop Code ID: "$CODE_MARBLE_PRESALEAIRDROP
    echo "CW721-base Code ID: "$CODE_CW721_BASE
    
    TXHASH=$(junod tx wasm instantiate $CODE_MARBLE_PRESALEAIRDROP '{"owner":"'$ADDR_MARBLE'", "pay_native": true, "airdrop": true, "native_denom":"'$DENOM'", "max_tokens":100000, "name":"MarbleNFT", "symbol":"MNFT", "token_code_id": '$CODE_CW721_BASE', "cw20_address":"'$TOKEN_MARBLE'", "royalty":0}' --admin $ADDR_MARBLE --label "MarbleAirdrop" $WALLET $TXFLAG -y --output json | jq -r '.txhash')
    echo $TXHASH
    CONTRACT_ADDR=""
    while [[ $CONTRACT_ADDR == "" ]]
    do
        sleep 3
        CONTRACT_ADDR=$(junod query tx $TXHASH $NODECHAIN --output json | jq -r '.logs[0].events[0].attributes[0].value')
    done
    echo $CONTRACT_ADDR
    echo $CONTRACT_ADDR > $FILE_AIRDROP_CONTRACT_ADDR
}

InstantiateMarketplace() { 
    echo "================================================="
    echo "Instantiate Marketplace Contract"
    CODE_MARBLE_COLLECTION=$(cat $FILE_CODE_MARBLE_COLLECTION)
    CODE_MARBLE_MARKETPLACE=$(cat $FILE_CODE_MARBLE_MARKETPLACE)
    CODE_CW721_BASE=$(cat $FILE_CODE_CW721_BASE)

    echo "Marketplace Code ID: "$CODE_MARBLE_MARKETPLACE
    echo "Collection Code ID: "$CODE_MARBLE_COLLECTION
    echo "CW721-base Code ID: "$CODE_CW721_BASE
    
    # Instantiate param in cosmwasm.tools
    # {
    #   "add_collection": {
    #     "owner": "juno1zzru8wptsc23z2lw9rvw4dq606p8fz0z6k6ggn",
    #     "max_tokens": 10000,
    #     "name": "Collection1",
    #     "symbol": "MNFT",
    #     "token_code_id": 360,
    #     "cw20_address": "juno15s50e6k9s8mac9cmrg2uq85cgw7fxxfh24xhr0chems2rjxsfjjs8kmuje",
    #     "royalty": 0,
    #     "uri": "ddd"
    #   }
    # }

    TXHASH=$(junod tx wasm instantiate $CODE_MARBLE_MARKETPLACE '{"cw721_base_code_id":'$CODE_CW721_BASE', "collection_code_id":'$CODE_MARBLE_COLLECTION'}' --label "MarbleMarketplace" --admin $ADDR_MARBLE $WALLET $TXFLAG -y --output json | jq -r '.txhash')
    echo $TXHASH
    CONTRACT_ADDR=""
    while [[ $CONTRACT_ADDR == "" ]]
    do
        sleep 3
        CONTRACT_ADDR=$(junod query tx $TXHASH $NODECHAIN --output json | jq -r '.logs[0].events[0].attributes[0].value')
    done
    echo $CONTRACT_ADDR
    echo $CONTRACT_ADDR > $FILE_MARKETPLACE_CONTRACT_ADDR
}

###################################################################################################
###################################################################################################
###################################################################################################
###################################################################################################

SetMerkleString() {
    
    MERKLEROOT=$(merkle-airdrop-cli generateRoot --file $AIRDROP_LIST_2)
    echo $MERKLEROOT
    echo $MERKLEROOT > $FILE_MERKLEROOT
    
    CONTRACT_ADDR=$(cat $FILE_AIRDROP_CONTRACT_ADDR)
    junod tx wasm execute $CONTRACT_ADDR '{"register_merkle_root":{"merkle_root":"'$MERKLEROOT'"}}' $WALLET $TXFLAG -y
}

###################################################################################################
###################################################################################################
###################################################################################################
###################################################################################################
AddCollection() {
    CONTRACT_MARKETPLACE=$(cat $FILE_MARKETPLACE_CONTRACT_ADDR)
    CODE_CW721_BASE=$(cat $FILE_CODE_CW721_BASE)


    # junod tx wasm execute $CONTRACT_MARKETPLACE '{"add_collection":{"owner": "'$ADDR_MARBLE'", "max_tokens": 1000000, "name": "Juno the Protector", "symbol": "MNFT","token_code_id": '$CODE_CW721_BASE',
    # "cw20_address": "'$TOKEN_MARBLE'",
    # "royalty": 0,
    # "uri": "https://marbledao.mypinata.cloud/ipfs/QmQWUou2j7uQ8MFXNg3exN9uaFgk4LgkDDZMCaWwWGMU3w"}}' $WALLET $TXFLAG -y

    # sleep 10
    
    junod tx wasm execute $CONTRACT_MARKETPLACE '{"add_collection":{"owner": "'$ADDR_MARBLE'", "max_tokens": 1000000, "name": "Laocoön The Priest", "symbol": "MNFT","token_code_id": '$CODE_CW721_BASE',
    "cw20_address": "'$TOKEN_MARBLE'",
    "royalty": 0,
    "uri": "https://marbledao.mypinata.cloud/ipfs/Qmf9jdbLfRbZQTfXu21u8UCj1Jp1y5GXHnBmMtmLnj1oUU"}}' $WALLET $TXFLAG -y
}

RemoveCollection() {
    CONTRACT_MARKETPLACE=$(cat $FILE_MARKETPLACE_CONTRACT_ADDR)
    junod tx wasm execute $CONTRACT_MARKETPLACE '{"remove_collection":{"id": 8}}' $WALLET $TXFLAG -y
}

ListCollection() {
    CONTRACT_MARKETPLACE=$(cat $FILE_MARKETPLACE_CONTRACT_ADDR)
    junod query wasm contract-state smart $CONTRACT_MARKETPLACE '{"list_collections":{}}' $NODECHAIN
}


#################################################################################
PrintWalletBalance() {
    echo "native balance"
    echo "========================================="
    junod query bank balances $ADDR_MARBLE $NODECHAIN
    echo "========================================="
}

#################################### End of Function ###################################################
if [[ $PARAM == "" ]]; then
    RustBuild
    Upload cw721_base
    sleep 3
    Upload marble_collection
    sleep 3
    Upload marble_marketplace
    sleep 3
    # Upload marble_presaleairdrop
    # InstantiatePresale
    # InstantiateAirdrop
    InstantiateMarketplace
    sleep 3
    AddCollection
    sleep 3
    ListCollection
else
    $PARAM $SUBPARAM
fi
