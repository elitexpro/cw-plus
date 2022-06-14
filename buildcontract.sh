#!/bin/bash

#Build Flag
PARAM=$1
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
TXFLAG=" $NODECHAIN --gas-prices 0.01$DENOM --gas auto --gas-adjustment 1.3"
WALLET="--from new_marble"

RELEASE="release/"

CW721WASMFILE=$RELEASE"cw721_base.wasm"
MARBLENFTWASMFILE=$RELEASE"marble_nft.wasm"
MARKETPLACEWASMFILE=$RELEASE"marketplace.wasm"

FILE_UPLOADHASH="uploadtx.txt"
FILE_CONTRACT_ADDR="contractaddr.txt"
FILE_CODE_ID="code.txt"

ADDR_ACHILLES="juno15fg4zvl8xgj3txslr56ztnyspf3jc7n9j44vhz"
ADDR_MARBLE="juno1zzru8wptsc23z2lw9rvw4dq606p8fz0z6k6ggn"

TOKEN_MARBLE_TEST="juno15s50e6k9s8mac9cmrg2uq85cgw7fxxfh24xhr0chems2rjxsfjjs8kmuje"
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
    RUSTFLAGS='-C link-arg=-s' cargo wasm

    mkdir release
    cp target/wasm32-unknown-unknown/$MARKETPLACEWASMFILE $MARKETPLACEWASMFILE
}

Upload() {
    echo "================================================="
    echo "Upload $MARKETPLACEWASMFILE"
    
    UPLOADTX=$(junod tx wasm store $MARKETPLACEWASMFILE $WALLET $TXFLAG --output json -y | jq -r '.txhash')
    echo "Upload txHash:"$UPLOADTX
    
    #save to FILE_UPLOADHASH
    echo $UPLOADTX > $FILE_UPLOADHASH
    echo "wrote last transaction hash to $FILE_UPLOADHASH"
}

#Read code from FILE_UPLOADHASH
GetCode() {
    echo "================================================="
    echo "Get code from transaction hash written on $FILE_UPLOADHASH"
    
    #read from FILE_UPLOADHASH
    TXHASH=$(cat $FILE_UPLOADHASH)
    echo "read last transaction hash from $FILE_UPLOADHASH"
    echo $TXHASH
    
    QUERYTX="junod query tx $TXHASH $NODECHAIN --output json"
	CODE_ID=$(junod query tx $TXHASH $NODECHAIN --output json | jq -r '.logs[0].events[-1].attributes[0].value')
	echo "Contract Code_id:"$CODE_ID

    #save to FILE_CODE_ID
    echo $CODE_ID > $FILE_CODE_ID
}

#Instantiate Contract
Instantiate() {
    echo "================================================="
    echo "Instantiate Contract"
    
    #read from FILE_CODE_ID
    CODE_ID=$(cat $FILE_CODE_ID)
    #mainnet code id: "cw721_base_code_id":388, "collection_code_id":389
    #testnet code id: "cw721_base_code_id":302, "collection_code_id":303
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
    TXHASH=$(junod tx wasm instantiate $CODE_ID '{"cw721_base_code_id":302, "collection_code_id":303}' --label "Marble Marketplace" --admin $ADDR_MARBLE $WALLET $TXFLAG -y --output json | jq -r '.txhash')
    echo $TXHASH
    echo $TXHASH > $FILE_UPLOADHASH

    sleep 15
    CONTRACT_ADDR=$(junod query tx $TXHASH $NODECHAIN --output json | jq -r '.logs[0].events[0].attributes[0].value')
    echo $CONTRACT_ADDR
    echo $CONTRACT_ADDR > $FILE_CONTRACT_ADDR
	echo "Contract Code_id:"$CODE_ID
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
    UploadAndGetCodeCW721Base
sleep 5
    Upload
sleep 8
    GetCode
sleep 10
    Instantiate
sleep 10
    GetContractAddress
sleep 5
    BatchMint
# sleep 5
#     SendFot
# sleep 5
#     Withdraw
sleep 5
    PrintConfig
sleep 5
    SetMerkleString
sleep 5
    Claim
# sleep 5
#     PrintWalletBalance
else
    $PARAM
fi

# OptimizeBuild
# Upload
# GetCode
# Instantiate
# GetContractAddress
# CreateEscrow
# TopUp

