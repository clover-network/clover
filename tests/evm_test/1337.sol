pragma solidity ^0.4.25;

contract HelloWorld {
  
   function test() public pure returns (uint) {
    return 1337;
  }
   
   mapping(address=>uint256) public balances;

   constructor(uint256 _balance) public {
	   balances[address(this)] = _balance;
   }

   function setValue(uint256 _value) public {
	   balances[msg.sender] = _value;
   }

   function getValueSelf() public view returns (uint256) {
	   return balances[msg.sender];
   }

   function getValue(address addr) public view returns (uint256) {
	   return balances[addr];
   }
}

