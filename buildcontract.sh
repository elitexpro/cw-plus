#!/bin/bash

#Build Flag
PARAM=$1
SUBPARAM=$2
####################################    Constants    ##################################################

#depends on mainnet or testnet
# NODE="--node https://rpc-juno.itastakers.com:443"
# CHAIN_ID=juno-1
# DENOM="ujuno"

##########################################################################################

NODE="--node https://rpc.juno.giansalex.dev:443"
CHAIN_ID=uni-3
DENOM="ujunox"

##########################################################################################
#not depends
NODECHAIN=" $NODE --chain-id $CHAIN_ID"
TXFLAG=" $NODECHAIN --gas-prices 0.001$DENOM --gas auto --gas-adjustment 1.3"
WALLET="--from new_marble"

RELEASE="release/"

CW721WASMFILE=$RELEASE"cw721_base.wasm"
PRESALEAIRDROPWASMFILE=$RELEASE"marble_presaleairdrop.wasm"
COLLECTIONWASMFILE=$RELEASE"marble_collection.wasm"
MARKETPLACEWASMFILE=$RELEASE"marble_marketplace.wasm"

FILE_CODE_CW721_BASE="code_cw721_base.txt"
FILE_CODE_MARBLE_COLLECTION="code_marble_collection.txt"
FILE_CODE_MARBLE_MARKETPLACE="code_marble_marketplace.txt"
FILE_CODE_MARBLE_PRESALEAIRDROP="code_marble_presaleairdrop.txt"

FILE_UPLOADHASH="uploadtx.txt"
FILE_PRESALE_CONTRACT_ADDR="contract_presale.txt"
FILE_AIRDROP_CONTRACT_ADDR="contract_airdrop.txt"
FILE_MARKETPLACE_CONTRACT_ADDR="contract_marketplace.txt"

ADDR_ACHILLES="juno15fg4zvl8xgj3txslr56ztnyspf3jc7n9j44vhz"
ADDR_MARBLE="juno1zzru8wptsc23z2lw9rvw4dq606p8fz0z6k6ggn"

TOKEN_MARBLE="juno15s50e6k9s8mac9cmrg2uq85cgw7fxxfh24xhr0chems2rjxsfjjs8kmuje"

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
    echo $CODE_ID > "code_"$SUBPARAM".txt"
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
    #     "token_code_id": 302,
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


Claim() {
    CONTRACT_MARBLENFT=$(cat $FILE_CONTRACT_ADDR)
    junod tx wasm execute $CONTRACT_MARBLENFT '{"claim":{"proof":[  "68c4141905c082cf699afa9ed1b8e4d2e3a278c1144cc784f1992493fc002edd",  "c3d63ecbcacef6c174fe18fead19c9ba640c27a15c7a5ef96f971ec816e26024"]}}' $WALLET $TXFLAG -y

    junod tx wasm execute $CONTRACT_MARBLENFT '{"claim":{"proof":[  "3506d5b5320f1f9bbecfd94147fa3a79e5eb093dbfc532a2826ecd03482b6020",  "c3d63ecbcacef6c174fe18fead19c9ba640c27a15c7a5ef96f971ec816e26024"]}}' --from marble2 $TXFLAG -y
}

Mint() {
    CONTRACT_MARBLENFT=$(cat $FILE_CONTRACT_ADDR)
    junod tx wasm execute $CONTRACT_MARBLENFT '{"mint":{"uri":"https://marbledao.mypinata.cloud/ipfs/QmQRi7Jg2wxKoTEj7813wksEf6Kxsa6YHTv4FVahBHii3A", "price":"1"}}' $WALLET $TXFLAG -y
}
BatchMint() {
    CONTRACT_MARBLENFT=$(cat $FILE_CONTRACT_ADDR)
    junod tx wasm execute $CONTRACT_MARBLENFT '{"batch_mint":{"uri":["0", "1", "2"], "price":["1", "2", "1"]}}' $WALLET $TXFLAG -y
}

BuyNative() {
    CONTRACT_MARBLENFT=$(cat $FILE_CONTRACT_ADDR)
    junod tx wasm execute $CONTRACT_MARBLENFT --amount "10ujuno" '{"buy_native":{}}' $WALLET $TXFLAG -y
}

UpdatePrice() {
    CONTRACT_MARBLENFT=$(cat $FILE_CONTRACT_ADDR)
    junod tx wasm execute $CONTRACT_MARBLENFT '{"update_price":{"token_id":[0], "price":["2"]}}' $WALLET $TXFLAG -y
}

ChangeOwner() {
    CONTRACT_MARBLENFT=$(cat $FILE_CONTRACT_ADDR)
    junod tx wasm execute $CONTRACT_MARBLENFT '{"change_owner":{"owner":"'$ADDR_ANDREW'"}}' $WALLET $TXFLAG -y
}

PrintConfig() {
    CONTRACT_MARBLENFT=$(cat $FILE_CONTRACT_ADDR)
    junod query wasm contract-state smart $CONTRACT_MARBLENFT '{"get_config":{}}' $NODECHAIN
}

GetPrice() {
    CONTRACT_MARBLENFT=$(cat $FILE_CONTRACT_ADDR)
    junod query wasm contract-state smart $CONTRACT_MARBLENFT '{"get_price":{"token_id":[0]}}' $NODECHAIN
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
    Upload marble_collection
    Upload marble_marketplace
    Upload marble_presaleairdrop
    InstantiatePresale
    InstantiateAirdrop
    InstantiateMarketplace
else
    $PARAM $SUBPARAM
fi
