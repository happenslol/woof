export const t = {
  heading: {
    title: () => `Woof`,
    subtitle: () => `The Woof Project`,
  },

  button: {
    label: () => `Click Me!`,
  },

  subscope: {
    heading: {
      title: () => `Subtitle`,
    },
  },
  something: {
    containing: {
      components: (values: { component: string }) =>
        `This is a string with a component in it: ${values.component}`,
      values: (values: { value: string }) =>
        `This is a string with a value in it: ${values.value}`,
    },
  },
};
