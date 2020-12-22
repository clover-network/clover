const fs = require('fs');
const Web3 = require('web3');
const web3 = new Web3('http://localhost:8545');
//const web3 = new Web3(new Web3.providers.HttpProvider("https://rinkeby.infura.io/v3/bd6d2612b6c8462a99385dc5c89cfd41")) //gas is : 367293

let accounts = [
  {
    // Develop 1
    address: '0xe6206C7f064c7d77C6d8e3eD8601c9AA435419cE',
    key: '0xa504b64992e478a6846670237a68ad89b6e42e90de0490273e28e74f084c03c8'
  },
]

async function send(transaction, acc) {
    let gas = await transaction.estimateGas({from: acc.address});
    console.log('gas is : ' + gas);
    let options = {
        to  : transaction._parent._address,
        data: transaction.encodeABI(),
        gas : gas,
        gasPrice: web3.utils.toWei("1", "gwei"),
    };
    let signedTransaction = await web3.eth.accounts.signTransaction(options, acc.key);
    return await web3.eth.sendSignedTransaction(signedTransaction.rawTransaction);
}

async function deploy(contractName, contractArgs, acc) {
    let abi = fs.readFileSync('./build/' + contractName + ".abi").toString();
    let bin = fs.readFileSync('./build/' + contractName + ".bin").toString();
    let contract = new web3.eth.Contract(JSON.parse(abi));
    let handle = await send(contract.deploy({data: "0x" + bin, arguments: contractArgs}), acc);
    console.log(`${contractName} contract deployed at address ${handle.contractAddress}`);
    return new web3.eth.Contract(JSON.parse(abi), handle.contractAddress);
}

async function run() {
    let myContract = await deploy("helloWorld", [1000], accounts[0]);
    let storage = await deploy("Storage", [], accounts[0]);

    console.log(myContract._address, storage._address)
    await send(myContract.methods.setStorage(storage._address), accounts[0]);
    let ret2 = await send(myContract.methods.setValue(9527), accounts[0]);
    console.log('result: ', ret2);
}

run();


