pragma solidity ^0.4.25;

interface  IStorage {
  function setValue(uint256 _value) external;
}

contract HelloWorld {
  
  function test() public pure returns (uint) {
    return 1337;
  }

  address public addStorage;
  mapping(address=>uint256) public balances;

  constructor(uint256 _balance) public {
    balances[address(this)] = _balance;
  }

  function setStorage(address _storage) public {
    addStorage = _storage;
  }

  function setValue(uint256 _value) public {
    balances[msg.sender] = _value;
    require(addStorage != address(0x0), "INVALID STORAGE");
    IStorage(addStorage).setValue(_value);
  }

  function getValueSelf() public view returns (uint256) {
    return balances[msg.sender];
  }

  function getValue(address addr) public view returns (uint256) {
    return balances[addr];
  }
}

contract Storage {
  mapping(address=>uint256) public data;

  function setValue(uint256 _value) public {
    data[msg.sender] = _value;
  }

  function getValue(address addr) public view returns (uint256) {
    return data[addr];
  }
}

