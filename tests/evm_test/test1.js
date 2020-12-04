const fs = require('fs');
const Web3 = require('web3');
const web3 = new Web3('http://localhost:8545');

let accounts = [
  {
    // Develop 1
    address: '0x6Be02d1d3665660d22FF9624b7BE0551ee1Ac91b',
    key: '99b3c12287537e38c90a9219d4cb074a89a16e9cdb20bf85728ebd97c343e342'
  },
]

async function send(transaction, acc) {
    let gas = await transaction.estimateGas({from: acc.address});
    let options = {
        to  : transaction._parent._address,
        data: transaction.encodeABI(),
        gas : gas
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
    let ret = await myContract.methods.test().call();
    let tx = myContract.methods.setValue(9527);
    let ret2 = await send(tx, accounts[0]);
    console.log('result: ', ret);
    console.log('result2: ', await myContract.methods.getValueSelf().call({from: accounts[0].address }));
}

run();


