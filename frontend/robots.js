// ... existing code ...

// Add a RobotsMenu component that renders a list of robot names
class RobotsMenu extends React.Component {
  render() {
    return (
      <div>
        {this.props.robotNames.map((name) => (
          <button key={name} onClick={() => this.props.selectRobot(name)}>{name}</button>
        ))}
      </div>
    );
  }
}

// ... existing code ...
