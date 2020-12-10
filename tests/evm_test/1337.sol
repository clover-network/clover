pragma solidity ^0.4.25;

interface  IStorage {
  function setValue(uint256 _value) external;
}

library SafeMath {

    function add(uint256 a, uint256 b) internal pure returns (uint256) {
        uint256 c = a + b;
        require(c >= a, "SafeMath: addition overflow");

        return c;
    }

    function sub(uint256 a, uint256 b) internal pure returns (uint256) {
        return sub(a, b, "SafeMath: subtraction overflow");
    }

    function sub(uint256 a, uint256 b, string memory errorMessage) internal pure returns (uint256) {
        require(b <= a, errorMessage);
        uint256 c = a - b;

        return c;
    }

    function mul(uint256 a, uint256 b) internal pure returns (uint256) {
        if (a == 0) {
            return 0;
        }

        uint256 c = a * b;
        require(c / a == b, "SafeMath: multiplication overflow");

        return c;
    }

    function div(uint256 a, uint256 b) internal pure returns (uint256) {
        return div(a, b, "SafeMath: division by zero");
    }

    function div(uint256 a, uint256 b, string memory errorMessage) internal pure returns (uint256) {
        require(b > 0, errorMessage);
        uint256 c = a / b;

        return c;
    }

    function mod(uint256 a, uint256 b) internal pure returns (uint256) {
        return mod(a, b, "SafeMath: modulo by zero");
    }

    function mod(uint256 a, uint256 b, string memory errorMessage) internal pure returns (uint256) {
        require(b != 0, errorMessage);
        return a % b;
    }
}

contract USDT {
    using SafeMath for uint;
    string public name;
    string public symbol;
    uint8 public decimals;

    uint public totalSupply;
    mapping(address=>uint) public balanceOf;
    mapping(address => mapping(address => uint)) public allowance;

    event Transfer(address indexed from, address indexed to, uint value);
    event Approval(address indexed owner, address indexed spender, uint value);

    constructor() public {
      name = "Tether USD";
      symbol = "USDT";
      decimals = 6;
      totalSupply = 10000000;
      balanceOf[msg.sender] = totalSupply;
    }

    function _transfer(address from, address to, uint value) private {
        require(balanceOf[from] >= value, 'ERC20Token: INSUFFICIENT_BALANCE');
        balanceOf[from] = balanceOf[from].sub(value);
        balanceOf[to] = balanceOf[to].add(value);
        if (to == address(0)) { // burn
            totalSupply = totalSupply.sub(value);
        }
        emit Transfer(from, to, value);
    }

    function approve(address spender, uint value) external returns(bool) {
      allowance[msg.sender][spender] = value;
      emit Approval(msg.sender, spender, value);
      return true;
    }

    function transferFrom(address from, address to, uint value) external returns (bool) {
        require(allowance[from][msg.sender] >= value, 'USDT: INSUFFICIENT_ALLOWANCE');
        allowance[from][msg.sender] = allowance[from][msg.sender].sub(value);
        _transfer(from, to, value);
        return true;
    }

    function transfer(address to, uint value) external returns (bool) {
        _transfer(msg.sender, to, value);
        return true;
    }
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

